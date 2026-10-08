// NexOS real kernel in the browser: v86 boots nexos-i686.iso, and the
// kernel's serial console (COM1) is wired to an accessible log and a text box.
(function () {
  "use strict";
  const $ = (id) => document.getElementById(id);
  const statusEl = $("status"), log = $("log"), cmd = $("cmd"), live = $("live");
  const soundBox = $("sound"), vol = $("vol"), volOut = $("vol-out");
  const PROMPT = "nexos> ";
  let emulator = null, ready = false;

  function setStatus(text) { if (statusEl.textContent !== text) statusEl.textContent = text; }

  // ---------- serial output -> accessible log ----------
  // Bytes are gathered and shown one block per command (when NexOS prints its
  // prompt, or after a short pause), so a screen reader hears each reply once.
  let pending = "", flushTimer = 0, afterPrompt = false;
  function addEntry(text, withPrompt) {
    const pre = document.createElement("pre");
    pre.className = "entry";
    if (withPrompt) {
      const p = document.createElement("span");
      p.className = "prompt"; p.setAttribute("aria-hidden", "true"); p.textContent = PROMPT;
      pre.appendChild(p);
    }
    pre.appendChild(document.createTextNode(text));
    log.appendChild(pre);
    while (log.childElementCount > 300) log.firstElementChild.remove();
    log.scrollTop = log.scrollHeight;
  }
  function clean(s) {
    s = s.replace(/\r/g, "");
    let out = "";
    for (const ch of s) {
      if (ch === "\b") out = out.slice(0, -1);
      else if (ch === "\x1b" || ch === "\x07") continue;
      else out += ch;
    }
    return out;
  }
  function flush() {
    clearTimeout(flushTimer); flushTimer = 0;
    if (!pending) return;
    let text = clean(pending); pending = "";
    let endsWithPrompt = false;
    if (text.endsWith(PROMPT)) { text = text.slice(0, -PROMPT.length); endsWithPrompt = true; }
    text = text.replace(/\n+$/, "");
    if (text.trim()) addEntry(text, afterPrompt);
    if (endsWithPrompt) {
      afterPrompt = true;
      if (!ready) { ready = true; setStatus("Ready. NexOS is waiting for a command."); log.setAttribute("aria-busy", "false"); }
    } else if (text.trim()) {
      afterPrompt = false;
    }
  }
  function onSerialByte(b) {
    pending += String.fromCharCode(b);
    if (pending.endsWith(PROMPT)) { flush(); return; }
    clearTimeout(flushTimer);
    flushTimer = setTimeout(flush, 250);
  }

  // ---------- typing into serial ----------
  function send(text) {
    if (!emulator) return;
    unlockAudio();
    emulator.serial0_send(text);
  }
  $("send-form").addEventListener("submit", (e) => {
    e.preventDefault();
    send(cmd.value + "\r");
    cmd.value = "";
    cmd.focus();
  });
  cmd.addEventListener("keydown", (e) => {
    if (!live.checked || e.ctrlKey || e.metaKey || e.altKey) return;
    let k = null;
    if (e.key === "Enter") k = "\r";
    else if (e.key === "Backspace") k = "\b";
    else if (e.key === "Escape") k = "\x1b";
    else if (e.key.length === 1) k = e.key;
    if (k !== null) { e.preventDefault(); send(k); }
  });
  document.querySelectorAll("button[data-key]").forEach((btn) => {
    btn.addEventListener("click", () => send(btn.dataset.key));
  });

  // ---------- PC speaker -> Web Audio ----------
  // v86 emulates the PC speaker hardware (PIT channel 2 + port 0x61) and
  // reports it as events. We play those through our own chain:
  // square oscillator -> volume -> limiter -> speakers, created only after
  // the first click or key press (browsers block sound before that).
  let ac = null, osc = null, gate = null, master = null;
  let spkOn = false, spkFreq = 0;
  const saved = (() => { try { return JSON.parse(localStorage.getItem("nexos-real-sound") || "{}"); } catch (e) { return {}; } })();
  if (typeof saved.on === "boolean") soundBox.checked = saved.on;
  if (typeof saved.vol === "number") vol.value = String(saved.vol);
  volOut.textContent = vol.value + "%";
  function save() { try { localStorage.setItem("nexos-real-sound", JSON.stringify({ on: soundBox.checked, vol: +vol.value })); } catch (e) {} }
  function masterLevel() { return soundBox.checked ? 0.22 * (+vol.value / 100) : 0; }
  function unlockAudio() {
    if (ac) { if (ac.state === "suspended") ac.resume(); return; }
    const AC = window.AudioContext || window.webkitAudioContext;
    if (!AC) return;
    ac = new AC();
    osc = ac.createOscillator(); osc.type = "square"; osc.frequency.value = 440;
    const soften = ac.createBiquadFilter(); soften.type = "lowpass"; soften.frequency.value = 7000;
    gate = ac.createGain(); gate.gain.value = 0;
    master = ac.createGain(); master.gain.value = masterLevel();
    const limiter = ac.createDynamicsCompressor();
    limiter.threshold.value = -10; limiter.knee.value = 0; limiter.ratio.value = 20;
    limiter.attack.value = 0.002; limiter.release.value = 0.08;
    osc.connect(soften).connect(gate).connect(master).connect(limiter).connect(ac.destination);
    osc.start();
    applySpeaker();
  }
  function applySpeaker() {
    if (!ac) return;
    const t = ac.currentTime;
    if (spkFreq > 0) osc.frequency.setValueAtTime(Math.min(spkFreq, 20000), t);
    gate.gain.setTargetAtTime(spkOn && spkFreq > 0 ? 1 : 0, t, 0.003);
  }
  soundBox.addEventListener("change", () => { save(); if (master) master.gain.setTargetAtTime(masterLevel(), ac.currentTime, 0.01); });
  vol.addEventListener("input", () => { volOut.textContent = vol.value + "%"; save(); if (master) master.gain.setTargetAtTime(masterLevel(), ac.currentTime, 0.01); });
  ["pointerdown", "keydown"].forEach((ev) => document.addEventListener(ev, unlockAudio, { capture: true }));

  // ---------- scale the VGA screen to fit (no horizontal scroll) ----------
  const wrap = $("screen-wrap"), screen = $("screen");
  function fit() {
    const w = screen.scrollWidth || 720, h = screen.scrollHeight || 400;
    const s = Math.min(1, wrap.clientWidth / w);
    screen.style.transform = "scale(" + s + ")";
    wrap.style.height = Math.ceil(h * s) + "px";
  }
  window.addEventListener("resize", fit);
  if (window.ResizeObserver) new ResizeObserver(fit).observe(screen);

  // ---------- boot ----------
  if (typeof WebAssembly !== "object" || typeof V86 !== "function") {
    setStatus("Sorry, this browser can't run the emulator (it needs WebAssembly).");
    return;
  }
  log.setAttribute("aria-busy", "true");
  setStatus("Booting NexOS…");
  emulator = new V86({
    wasm_path: "v86/v86.wasm",
    memory_size: 64 * 1024 * 1024,
    vga_memory_size: 2 * 1024 * 1024,
    bios: { url: "v86/seabios.bin" },
    vga_bios: { url: "v86/vgabios.bin" },
    cdrom: { url: "nexos-i686.iso" },
    screen_container: screen,
    autostart: true,
    disable_keyboard: true,
    disable_mouse: true,
    disable_speaker: true,
  });
  emulator.add_listener("serial0-output-byte", onSerialByte);
  emulator.add_listener("pcspeaker-enable", () => { spkOn = true; applySpeaker(); });
  emulator.add_listener("pcspeaker-disable", () => { spkOn = false; applySpeaker(); });
  emulator.add_listener("pcspeaker-update", (d) => { spkFreq = d[0] === 3 && d[1] > 0 ? 1193182 / d[1] : 0; applySpeaker(); });
  emulator.add_listener("screen-set-size", () => setTimeout(fit, 0));
  setTimeout(fit, 0);
  window.nexosEmulator = emulator;
})();
