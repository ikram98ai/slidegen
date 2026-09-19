/* Data Town kit — tiny helpers shared by every chapter page of every book.
   Usage: <script src="book.js"></script><script src="../../assets/kit.js"></script> then K.page(3) and build sims with K.h / K.s / K.tween. */
(function () {
  const NS = "http://www.w3.org/2000/svg";
  const reduced = window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;

  /* The book manifest (books/<slug>/book.js) sets window.BOOK before this file loads. */
  const BOOK = window.BOOK || { slug: "book", title: "Library", chapters: [], built: 0 };
  const CHAPTERS = BOOK.chapters || [];
  /** Chapters 1..BUILT have pages; the rest are shown as "coming soon". */
  const BUILT = BOOK.built == null ? CHAPTERS.length : BOOK.built;

  /* ---- DOM helpers ---- */
  function apply(el, attrs, isSvg) {
    for (const [k, v] of Object.entries(attrs || {})) {
      if (v === undefined || v === null || v === false) continue;
      if (k === "class") el.setAttribute("class", v);
      else if (k === "style" && typeof v === "object") Object.assign(el.style, v);
      else if (k === "text") el.textContent = v;
      else if (k === "html") el.innerHTML = v;
      else if (k.startsWith("on") && typeof v === "function") el.addEventListener(k.slice(2), v);
      else el.setAttribute(k, v === true ? "" : v);
    }
    return el;
  }
  function add(el, kids) {
    for (const c of kids.flat()) {
      if (c === null || c === undefined || c === false) continue;
      el.appendChild(typeof c === "string" || typeof c === "number" ? document.createTextNode(String(c)) : c);
    }
    return el;
  }
  /** HTML element: h('button', {class:'btn', onclick}, 'Go') */
  const h = (tag, attrs, ...kids) => add(apply(document.createElement(tag), attrs), kids);
  /** SVG element: s('rect', {x, y, width, height, class:'sv-box'}) */
  const s = (tag, attrs, ...kids) => add(apply(document.createElementNS(NS, tag), attrs, true), kids);
  const $ = (sel, root = document) => root.querySelector(sel);
  const $$ = (sel, root = document) => [...root.querySelectorAll(sel)];
  const clear = (el) => { while (el.firstChild) el.removeChild(el.firstChild); return el; };

  /* ---- motion ---- */
  const ease = {
    linear: (t) => t,
    out: (t) => 1 - Math.pow(1 - t, 3),
    inOut: (t) => (t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2),
    back: (t) => { const c = 1.7; return 1 + (c + 1) * Math.pow(t - 1, 3) + c * Math.pow(t - 1, 2); },
  };
  const speed = { value: 1 };
  const sleep = (ms) => new Promise((r) => setTimeout(r, reduced ? Math.min(ms, 40) : ms / speed.value));
  /** tween(ms, t => {...}, ease.inOut) → Promise, resolves when done */
  function tween(ms, fn, e = ease.inOut) {
    return new Promise((resolve) => {
      const dur = reduced ? 1 : ms / speed.value;
      const t0 = performance.now();
      function frame(now) {
        const t = Math.min(1, (now - t0) / dur);
        fn(e(t));
        if (t < 1) requestAnimationFrame(frame); else resolve();
      }
      requestAnimationFrame(frame);
    });
  }
  const lerp = (a, b, t) => a + (b - a) * t;
  /** Move an SVG element (use a <g>) from point a to b: fly(g, [x,y], [x,y], ms) */
  function fly(el, a, b, ms = 700, e = ease.inOut) {
    return tween(ms, (t) => el.setAttribute("transform", `translate(${lerp(a[0], b[0], t)},${lerp(a[1], b[1], t)})`), e);
  }
  /** Move along several points: path(g, [[x,y],[x,y],...], ms per leg) */
  async function path(el, pts, ms = 600) {
    for (let i = 1; i < pts.length; i++) await fly(el, pts[i - 1], pts[i], ms, i === 1 ? ease.inOut : ease.inOut);
  }
  /** A pausable animation loop: const L = loop((dt, t) => {...}); L.start(); L.stop(); */
  function loop(fn) {
    let raf = 0, last = 0, running = false, elapsed = 0;
    function frame(now) {
      const dt = Math.min(64, now - last) * speed.value; last = now; elapsed += dt;
      fn(dt, elapsed);
      if (running) raf = requestAnimationFrame(frame);
    }
    return {
      start() { if (running) return; running = true; last = performance.now(); raf = requestAnimationFrame(frame); },
      stop() { running = false; cancelAnimationFrame(raf); },
      get running() { return running; },
    };
  }
  /** Run start() while el is on screen and stop() when it scrolls away (keeps pages light). */
  function whenVisible(el, start, stop) {
    if (!("IntersectionObserver" in window)) { start(); return; }
    new IntersectionObserver((es) => es.forEach((e) => (e.isIntersecting ? start() : stop && stop())), { threshold: 0.05 }).observe(el);
  }

  /* ---- UI bits ---- */
  /** Segmented control: seg(['A','B'], 0, i => ...) */
  function seg(labels, active, onChange, id) {
    const wrap = h("div", { class: "seg", role: "group", id });
    labels.forEach((label, i) => {
      const b = h("button", { type: "button", "aria-pressed": String(i === active) }, label);
      b.addEventListener("click", () => {
        $$("button", wrap).forEach((x, j) => x.setAttribute("aria-pressed", String(j === i)));
        onChange(i);
      });
      wrap.appendChild(b);
    });
    return wrap;
  }
  /** Slider: slider({id, label, min, max, step, value, format, onInput}) */
  function slider(o) {
    const out = h("output", { for: o.id }, (o.format || String)(o.value));
    const input = h("input", { type: "range", id: o.id, min: o.min, max: o.max, step: o.step || 1, value: o.value });
    input.addEventListener("input", () => { const v = Number(input.value); out.textContent = (o.format || String)(v); o.onInput && o.onInput(v); });
    return h("div", { class: "slider" }, h("label", { for: o.id }, o.label), input, out);
  }
  /** Stat tile that you can update: const st = stat('Queries', 0); st.set(5, 'good') */
  function stat(label, value, tone) {
    const v = h("span", { class: "v" }, String(value));
    const el = h("div", { class: "stat" + (tone ? " " + tone : "") }, h("span", { class: "k" }, label), v);
    el.set = (val, t) => { v.textContent = String(val); el.className = "stat" + (t ? " " + t : ""); };
    return el;
  }
  let toastEl;
  function toast(msg) {
    if (!toastEl) { toastEl = h("div", { class: "toast", role: "status", "aria-live": "polite" }); document.body.appendChild(toastEl); }
    toastEl.textContent = msg; toastEl.classList.add("show");
    clearTimeout(toastEl._t); toastEl._t = setTimeout(() => toastEl.classList.remove("show"), 2200);
  }

  /* ---- progress (per-viewer convenience only) ---- */
  const KEY = `datatown-progress-v1:${BOOK.slug}`;
  function getProgress() { try { return JSON.parse(localStorage.getItem(KEY) || "{}"); } catch (e) { return {}; } }
  function setDone(n) { try { const p = getProgress(); p[n] = true; localStorage.setItem(KEY, JSON.stringify(p)); } catch (e) {} }

  /** Quiz: quiz(container, [{q, options:[..], answer: 1, why:'...'}], chapterNumber) */
  function quiz(root, items, chapter) {
    let right = 0, answered = 0;
    const score = h("div", { class: "score", "aria-live": "polite" }, `0 / ${items.length} answered`);
    items.forEach((it, qi) => {
      const why = h("p", { class: "why", hidden: true }, it.why || "");
      const opts = h("div", { class: "opts" });
      it.options.forEach((label, oi) => {
        const b = h("button", { type: "button", class: "opt", id: `q${chapter}-${qi}-${oi}` }, label);
        b.addEventListener("click", () => {
          if (opts.dataset.done) return;
          opts.dataset.done = "1"; answered++;
          const buttons = $$(".opt", opts);
          buttons[it.answer].classList.add("right");
          if (oi === it.answer) right++; else b.classList.add("wrong");
          why.hidden = false;
          why.prepend(h("strong", {}, oi === it.answer ? "Yes! " : "Not quite. "));
          score.textContent = `${right} / ${items.length} correct` + (answered === items.length ? (right === items.length ? " — perfect!" : " — nice work") : "");
          if (answered === items.length) { setDone(chapter); renderDots(chapter); toast(`Chapter ${chapter} marked as explored`); }
        });
        opts.appendChild(b);
      });
      root.appendChild(h("div", { class: "q" }, h("h3", {}, `${qi + 1}. ${it.q}`), opts, why));
    });
    root.appendChild(score);
  }

  /* ---- page chrome ---- */
  const brandSvg = () =>
    s("svg", { class: "brand-mark", viewBox: "0 0 26 26", "aria-hidden": "true" },
      s("rect", { x: 1, y: 9, width: 11, height: 16, rx: 2.5, class: "sv-fill-blue" }),
      s("rect", { x: 14, y: 3, width: 11, height: 22, rx: 2.5, class: "sv-fill-ink" }),
      s("rect", { x: 4, y: 3, width: 5, height: 5, rx: 1.5, class: "sv-fill-sun" }),
      s("rect", { x: 17, y: 7, width: 5, height: 3, rx: 1, class: "sv-fill-surface" }),
      s("rect", { x: 17, y: 13, width: 5, height: 3, rx: 1, class: "sv-fill-surface" }));

  function renderDots(current) {
    const dots = $(".dots"); if (!dots) return;
    const p = getProgress();
    clear(dots);
    CHAPTERS.forEach((c) => {
      const ready = c.n <= BUILT;
      dots.appendChild(ready
        ? h("a", { href: file(c.n), title: `${c.n}. ${c.title}`, "aria-label": `Chapter ${c.n}: ${c.title}`, class: (c.n === current ? "here " : "") + (p[c.n] ? "done" : "") })
        : h("span", { class: "soon", title: `${c.n}. ${c.title} — coming soon` }));
    });
  }
  const file = (n) => `ch${String(n).padStart(2, "0")}.html`;

  /** Call once per chapter page: K.page(n). Fills #topbar and #chapter-end. */
  function page(n) {
    const c = CHAPTERS[n - 1];
    const prev = CHAPTERS[n - 2], next = n < BUILT ? CHAPTERS[n] : null, upcoming = CHAPTERS[n];
    const top = $("#topbar");
    if (top) {
      top.className = "topbar";
      top.appendChild(h("div", { class: "wrap" },
        h("a", { class: "navbtn wide", href: "../../index.html", title: "All books" }, "Books"),
        h("a", { class: "brand", href: "index.html" }, brandSvg(), BOOK.title),
        h("span", { class: "where" }, `Ch ${n} · ${c.title}`),
        h("span", { class: "spacer" }),
        h("nav", { class: "dots", "aria-label": "Chapters" }),
        h("a", { class: "navbtn", href: prev ? file(prev.n) : "#", "aria-label": "Previous chapter", "aria-disabled": String(!prev) }, "←"),
        h("a", { class: "navbtn", href: next ? file(next.n) : "#", "aria-label": "Next chapter", "aria-disabled": String(!next) }, "→")));
      renderDots(n);
    }
    const end = $("#chapter-end");
    if (end) {
      end.className = "chapter-end";
      end.appendChild(h("div", { class: "wrap" },
        h("div", { class: "end-row" },
          h("a", { class: "btn", href: "index.html" }, "← All chapters"),
          h("button", { class: "btn good", type: "button", onclick: () => { setDone(n); renderDots(n); toast(`Chapter ${n} marked as explored`); } }, "✓ Mark chapter explored")),
        next
          ? h("a", { class: "next-card", href: file(next.n) }, h("span", { class: "num" }, String(next.n).padStart(2, "0")),
              h("span", {}, h("div", { class: "s" }, "Next stop"), h("div", { class: "t" }, next.title), h("div", { class: "s" }, next.kid)),
              h("span", { class: "arrow", "aria-hidden": "true" }, "→"))
          : h("a", { class: "next-card", href: "index.html" }, h("span", { class: "num" }, upcoming ? "…" : "✓"),
              h("span", {},
                h("div", { class: "s" }, upcoming ? "Coming soon" : "The end"),
                h("div", { class: "t" }, upcoming ? `${upcoming.n}. ${upcoming.title}` : "You finished every chapter!"),
                h("div", { class: "s" }, upcoming ? "Not built yet — head back to the map for the chapters that are ready." : "Head back to the map to revisit any chapter.")),
              h("span", { class: "arrow", "aria-hidden": "true" }, "→"))));
    }
    document.title = document.title || `Data Town ${n}`;
  }

  /* ---- drawing vocabulary: consistent pictures across chapters (all centred on cx, cy) ---- */
  const draw = {
    /** text label */
    label(x, y, text, cls = "sv-label", anchor = "middle") { return s("text", { x, y, class: cls, "text-anchor": anchor }, text); },
    /** database cylinder (lilac) */
    db(cx, cy, w = 70, hgt = 80, label, cls = "sv-db") {
      const rx = w / 2, ry = Math.max(6, w * 0.16), x0 = cx - rx, top = cy - hgt / 2 + ry, bot = cy + hgt / 2 - ry;
      return s("g", {},
        s("path", { class: cls, d: `M${x0},${top} V${bot} A${rx},${ry} 0 0 0 ${cx + rx},${bot} V${top}` }),
        s("ellipse", { class: cls, cx, cy: top, rx, ry }),
        s("path", { d: `M${x0},${cy - 2} A${rx},${ry} 0 0 0 ${cx + rx},${cy - 2}`, fill: "none", class: "sv-wire-solid", style: { stroke: "var(--lilac)", opacity: 0.45 } }),
        label ? draw.label(cx, cy + hgt / 2 + 18, label) : null);
    },
    /** server / service box (blue) with slots + status light. g.light lets you recolor the LED. */
    server(cx, cy, w = 80, hgt = 90, label, cls = "sv-server") {
      const x = cx - w / 2, y = cy - hgt / 2;
      const light = s("circle", { cx: x + w - 14, cy: y + 14, r: 5, class: "sv-fill-mint" });
      const g = s("g", {},
        s("rect", { x, y, width: w, height: hgt, rx: 12, class: cls }),
        ...[0, 1, 2].slice(0, Math.max(1, Math.floor((hgt - 24) / 16))).map((i) => s("rect", { x: x + 12, y: y + 30 + i * 16, width: w - 24, height: 7, rx: 3.5, class: "sv-fill-surface", style: { opacity: 0.9 } })),
        light,
        label ? draw.label(cx, cy + hgt / 2 + 18, label) : null);
      g.light = light;
      return g;
    },
    /** a person (use a fill class for color) */
    person(cx, cy, cls = "sv-fill-ink", scale = 1) {
      return s("g", { transform: `translate(${cx},${cy}) scale(${scale})` },
        s("circle", { cx: 0, cy: -14, r: 9, class: cls }),
        s("path", { d: "M-15,16 C-15,-2 15,-2 15,16 Z", class: cls }));
    },
    /** phone / frontend */
    phone(cx, cy, label) {
      return s("g", {},
        s("rect", { x: cx - 18, y: cy - 32, width: 36, height: 64, rx: 8, class: "sv-box" }),
        s("rect", { x: cx - 12, y: cy - 24, width: 24, height: 40, rx: 3, class: "sv-panel" }),
        s("circle", { cx, cy: cy + 24, r: 2.5, class: "sv-fill-line" }),
        label ? draw.label(cx, cy + 50, label) : null);
    },
    /** a data parcel — returns a <g> positioned at 0,0; move it with K.fly */
    parcel(cls = "sv-data", size = 16, text) {
      const g = s("g", { transform: "translate(-100,-100)" },
        s("rect", { x: -size / 2, y: -size / 2, width: size, height: size, rx: size * 0.25, class: cls }),
        text ? s("text", { x: 0, y: 4, class: "sv-label", "text-anchor": "middle", style: { fill: "var(--ink)", fontSize: "10px", fontWeight: 700 } }, text)
             : s("rect", { x: -size / 2, y: -1.5, width: size, height: 3, class: "sv-fill-surface", style: { opacity: 0.55 } }));
      return g;
    },
    /** straight arrow with head */
    arrow(x1, y1, x2, y2, cls = "sv-wire-solid") {
      const a = Math.atan2(y2 - y1, x2 - x1), L = 9;
      return s("g", {},
        s("line", { x1, y1, x2, y2, class: cls }),
        s("path", { d: `M${x2},${y2} L${x2 - L * Math.cos(a - 0.45)},${y2 - L * Math.sin(a - 0.45)} L${x2 - L * Math.cos(a + 0.45)},${y2 - L * Math.sin(a + 0.45)} Z`, class: "sv-fill-line", style: { fill: "var(--line-strong)" } }));
    },
    /** speech bubble whose tail points at (cx, cy); sizes itself to the text. g.say('hi') / g.say('') */
    bubble(cx, cy, minW = 60, bounds = [0, 10000]) {
      const t = s("text", { x: cx, y: cy - 22, class: "sv-text", "text-anchor": "middle", style: { fontSize: "13px", fontWeight: 700 } }, "");
      const r = s("rect", { x: cx - minW / 2, y: cy - 42, width: minW, height: 30, rx: 10, class: "sv-box" });
      const g = s("g", { style: { opacity: 0, transition: "opacity .25s" }, "aria-hidden": "true" },
        r, s("path", { d: `M${cx - 7},${cy - 13.5} L${cx},${cy - 4} L${cx + 7},${cy - 13.5} Z`, class: "sv-fill-surface" }), t);
      g.say = (text) => {
        t.textContent = text || "";
        const w = Math.max(minW, (text || "").length * 7.1 + 24);
        const x = Math.min(Math.max(cx - w / 2, bounds[0]), bounds[1] - w);
        r.setAttribute("x", x); r.setAttribute("width", w); t.setAttribute("x", x + w / 2);
        g.style.opacity = text ? 1 : 0;
      };
      return g;
    },
    /** rounded tag with text, positioned at 0,0 — move it with K.fly */
    tag(text, cls = "sv-data-soft") {
      const w = Math.max(28, String(text).length * 7.4 + 18);
      return s("g", { transform: "translate(-200,-200)" },
        s("rect", { x: -w / 2, y: -11, width: w, height: 22, rx: 11, class: cls }),
        s("text", { x: 0, y: 4, class: "sv-label", "text-anchor": "middle", style: { fill: "var(--ink)", fontWeight: 700 } }, String(text)));
    },
  };

  window.K = { BOOK, CHAPTERS, BUILT, h, s, $, $$, clear, tween, fly, path, loop, sleep, lerp, ease, speed, reduced, whenVisible, seg, slider, stat, toast, quiz, page, getProgress, setDone, file, draw };
})();
