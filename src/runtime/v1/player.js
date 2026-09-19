/* Slidegen chapter player — renders a SceneSpec manifest with the shared kit.
   Speaks to the host only via postMessage so chapter CSS/JS never enter the parent. */
(function () {
  const M = window.SLIDEGEN_CHAPTER;
  const K = window.K;
  if (!M || !K) return;

  const { h, $, clear, draw, s, slider, quiz } = K;

  function post(type, extra) {
    try {
      parent.postMessage(Object.assign({ source: "slidegen", type: type }, extra || {}), "*");
    } catch (e) { /* host may be missing */ }
  }

  function reportHeight() {
    post("slidegen:resize", { height: Math.max(document.documentElement.scrollHeight, 400) });
  }

  function citeButton(c) {
    const label = c.printed_page != null ? "p. " + c.printed_page : "PDF p. " + c.pdf_page;
    const tip = h("span", { class: "cite-tip", role: "tooltip" }, c.quote || c.paragraph_id || "");
    return h("button", {
      type: "button",
      class: "cite",
      title: c.quote || c.paragraph_id,
      "aria-label": "Source " + label,
      onclick: function () {
        post("slidegen:citation", {
          pdf_page: c.pdf_page,
          printed_page: c.printed_page,
          paragraph_id: c.paragraph_id,
          quote: c.quote,
        });
      },
    }, label, tip);
  }

  function playDepth(scene) {
    const audio = document.getElementById("sg-audio");
    if (!audio || !scene.depth || !scene.depth.audio_url) return;
    if (audio.dataset.scene === scene.id && !audio.paused) {
      audio.pause();
      return;
    }
    audio.dataset.scene = scene.id;
    audio.src = scene.depth.audio_url;
    audio.play().catch(function () { /* autoplay blocked until a click */ });
    post("slidegen:voice", { scene_id: scene.id, play: true });
  }

  function depthBlock(scene) {
    const panel = h("div", { class: "depth-panel", hidden: true, id: "depth-" + scene.id },
      h("div", { class: "depth-head" },
        h("strong", {}, "In depth"),
        scene.depth && scene.depth.audio_url
          ? h("button", { type: "button", class: "btn small", onclick: function () { playDepth(scene); } }, "Listen")
          : null),
      h("p", {}, (scene.depth && scene.depth.text) || ""));
    const btn = h("button", {
      type: "button",
      class: "icon-btn",
      title: "In-depth explanation",
      "aria-expanded": "false",
      "aria-controls": "depth-" + scene.id,
      onclick: function () {
        const open = panel.hidden;
        panel.hidden = !open;
        btn.setAttribute("aria-expanded", String(open));
        if (open) playDepth(scene);
        reportHeight();
      },
    }, "i");
    return { btn: btn, panel: panel };
  }

  function renderSteps(params, stage) {
    const steps = params.steps || [];
    let i = 0;
    const caption = h("div", { class: "caption", "aria-live": "polite" }, steps[0] ? (steps[0].caption || steps[0].title || "") : "");
    const buttons = h("div", { class: "step-list" });
    function show(n) {
      i = n;
      caption.textContent = steps[n] ? (steps[n].caption || steps[n].title || "") : "";
      Array.prototype.forEach.call(buttons.children, function (b, idx) {
        b.setAttribute("aria-pressed", String(idx === n));
      });
    }
    steps.forEach(function (step, idx) {
      buttons.appendChild(h("button", {
        type: "button",
        class: "btn small",
        "aria-pressed": String(idx === 0),
        onclick: function () { show(idx); },
      }, step.title || ("Step " + (idx + 1))));
    });
    stage.appendChild(h("div", { class: "stage-head" }, h("span", { class: "stage-title" }, params.title || "Try it")));
    stage.appendChild(buttons);
    stage.appendChild(caption);
  }

  function renderTwoLane(params, stage) {
    const left = params.left || {};
    const right = params.right || {};
    function lane(side) {
      const items = side.points || side.items || [];
      return h("div", { class: "lane" },
        h("h4", {}, side.title || ""),
        h("ul", {}, items.map(function (p) { return h("li", {}, String(p)); })));
    }
    stage.appendChild(h("div", { class: "stage-head" }, h("span", { class: "stage-title" }, params.title || "Compare")));
    stage.appendChild(h("div", { class: "lane-grid" }, lane(left), lane(right)));
  }

  function renderTradeoff(params, stage) {
    const left = params.left || {};
    const right = params.right || {};
    stage.appendChild(h("div", { class: "stage-head" }, h("span", { class: "stage-title" }, params.title || "Trade-off")));
    stage.appendChild(h("div", { class: "tradeoff" },
      h("div", { class: "side" }, h("h4", {}, left.title || ""), h("p", {}, left.text || "")),
      h("span", { class: "vs" }, "vs"),
      h("div", { class: "side" }, h("h4", {}, right.title || ""), h("p", {}, right.text || ""))));
  }

  function renderFlow(params, stage) {
    const nodes = params.nodes || [];
    const row = h("div", { class: "flow" });
    nodes.forEach(function (label, idx) {
      if (idx) row.appendChild(h("span", { class: "arrow", "aria-hidden": "true" }, "→"));
      row.appendChild(h("span", { class: "node" }, String(label)));
    });
    stage.appendChild(h("div", { class: "stage-head" }, h("span", { class: "stage-title" }, params.title || "Flow")));
    const svgWrap = h("div", { class: "stage-scroll" });
    if (nodes.length && draw) {
      const svg = s("svg", { viewBox: "0 0 640 160", role: "img", "aria-label": nodes.join(" to ") });
      const gap = 640 / (nodes.length + 1);
      nodes.forEach(function (label, idx) {
        const x = gap * (idx + 1);
        svg.appendChild(draw.server(x, 70, 88, 70, String(label)));
        if (idx) svg.appendChild(draw.arrow(gap * idx + 44, 70, x - 44, 70));
      });
      svgWrap.appendChild(svg);
    } else {
      svgWrap.appendChild(row);
    }
    stage.appendChild(svgWrap);
    if (params.caption) stage.appendChild(h("div", { class: "caption" }, params.caption));
  }

  function renderSlider(params, stage, sceneId) {
    const captions = params.captions || [];
    const cap = h("div", { class: "caption", "aria-live": "polite" }, captions[0] || "");
    stage.appendChild(h("div", { class: "stage-head" }, h("span", { class: "stage-title" }, params.title || "Compare")));
    stage.appendChild(h("div", { class: "controls" },
      slider({
        id: "sg-slider-" + sceneId,
        label: params.label || "Amount",
        min: 0,
        max: Math.max(captions.length - 1, 1),
        step: 1,
        value: 0,
        format: function (v) { return (params.left_label || "") + " → " + (params.right_label || "") + " (" + v + ")"; },
        onInput: function (v) { cap.textContent = captions[v] || captions[captions.length - 1] || ""; },
      })));
    stage.appendChild(cap);
  }

  function renderViz(scene, stage) {
    const viz = scene.viz || { type: "steps", params: {} };
    const params = viz.params || {};
    switch (viz.type) {
      case "two_lane":
        renderTwoLane(params, stage);
        break;
      case "tradeoff":
        renderTradeoff(params, stage);
        break;
      case "flow":
        renderFlow(params, stage);
        break;
      case "slider_compare":
        renderSlider(params, stage, scene.id);
        break;
      case "quiz":
        stage.appendChild(h("div", { class: "stage-head" }, h("span", { class: "stage-title" }, "Check")));
        {
          const box = h("div", { class: "quiz", style: { padding: "12px" } });
          quiz(box, scene.quiz || params.items || [], M.order_index || 1);
          stage.appendChild(box);
        }
        break;
      default:
        renderSteps(params, stage);
    }
  }

  function renderScene(scene, index) {
    const depth = depthBlock(scene);
    const cites = (scene.citations || []).map(citeButton);
    const stage = h("div", { class: "stage" });
    renderViz(scene, stage);
    return h("section", { class: "scene", id: scene.id },
      h("div", { class: "wrap scene-grid" },
        h("div", { class: "copy" },
          h("span", { class: "eyebrow" }, "Scene " + (index + 1)),
          h("h2", {}, scene.title || ""),
          h("p", { class: "story" }, scene.kid_summary || ""),
          h("div", { style: { display: "flex", alignItems: "center", gap: "10px", flexWrap: "wrap" } },
            depth.btn,
            cites.length ? h("div", { class: "cite-row" }, cites) : null),
          depth.panel),
        stage));
  }

  function render() {
    document.title = M.title || "Chapter";
    const top = $("#topbar");
    if (top) {
      top.className = "topbar";
      clear(top);
      top.appendChild(h("div", { class: "wrap" },
        h("span", { class: "brand" }, M.book_title || "Chapter"),
        h("span", { class: "where" }, M.title || "")));
    }

    const root = $("#chapter-root");
    if (!root) return;
    clear(root);
    root.appendChild(h("section", { class: "wrap hero" },
      h("div", {},
        h("span", { class: "ch-tag" }, h("span", { class: "num" }, String((M.order_index || 0) + 1)), "Chapter"),
        h("h1", {}, M.title || ""),
        h("p", { class: "lede" }, M.kid_lede || ""),
        h("nav", { class: "toc" }, (M.scenes || []).map(function (sc) {
          return h("a", { href: "#" + sc.id }, sc.title || sc.id);
        })))));
    (M.scenes || []).forEach(function (scene, i) {
      root.appendChild(renderScene(scene, i));
    });
    if (!document.getElementById("sg-audio")) {
      document.body.appendChild(h("audio", { id: "sg-audio", preload: "none" }));
    }
    reportHeight();
  }

  window.addEventListener("message", function (ev) {
    const data = ev.data || {};
    if (data.type === "slidegen:setTheme" && data.theme) {
      document.documentElement.setAttribute("data-theme", data.theme);
    }
  });

  render();
  post("slidegen:ready", { chapter_id: M.chapter_id, height: document.documentElement.scrollHeight });
  if (window.ResizeObserver) {
    new ResizeObserver(reportHeight).observe(document.body);
  }
})();
