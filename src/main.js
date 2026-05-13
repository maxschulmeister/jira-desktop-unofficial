// main.js
import { openWebsiteInNewWindow } from "./utils/window_utils.js";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getVersion } from "@tauri-apps/api/app";

const isTauri = window.__TAURI__ !== undefined;

document.addEventListener("DOMContentLoaded", () => {
  // ── DOM refs ──────────────────────────────────────────
  const instanceName = document.getElementById("instanceName");
  const instanceUrl = document.getElementById("instanceUrl");
  const saveBtn = document.getElementById("saveBtn");
  const cancelBtn = document.getElementById("cancelBtn");
  const addNewBtn = document.getElementById("addNewBtn");
  const urlInputContainer = document.getElementById("url-input-container");
  const savedUrlsContainer = document.getElementById("saved-urls-container");
  const instancesList = document.getElementById("instancesList");
  const errorDiv = document.getElementById("error");
  const githubLink = document.getElementById("github-link");
  const maintainerLink = document.getElementById("maintainer-link");
  const versionApp = document.getElementById("version-app");

  const bgSettingsBtn = document.getElementById("bgSettingsBtn");
  const bgModal = document.getElementById("bgModal");
  const modalBackdrop = bgModal?.querySelector(".modal-backdrop");
  const closeModal = bgModal?.querySelector(".close");
  const bgSelect = document.getElementById("bgSelect");
  const overlayOpacity = document.getElementById("overlayOpacity");
  const opacityValue = document.getElementById("opacityValue");
  const applyBgBtn = document.getElementById("applyBgBtn");

  if (!instanceName || !saveBtn || !instancesList) {
    console.error("Critical DOM elements not found.");
    return;
  }

  // ── Background ────────────────────────────────────────
  let currentBg = { preset: "default", opacity: 50 };

  const bgPresets = {
    default: "bg.jpg",
    abstract: "pexels-codioful-6985268.jpg",
    city: "pexels-mathew-liang-416678-3013999.jpg",
    pastel: "pexels-darya-grey_owl-132130036-11478290.jpg",
    nature: "pexels-kovalskiolga-13789391.jpg",
    blue: "pexels-giancarlo-rojas-2002126-5660082.jpg",
  };

  function getImageUrl(relativePath) {
    if (import.meta.env.DEV) {
      return `./assets/${relativePath}`;
    }

    return `${relativePath}`;
  }

  function applyBackground(preset, opacity) {
    const bgUrl = getImageUrl(bgPresets[preset] || bgPresets.default);

    document.body.style.backgroundImage = `url('${bgUrl}')`;
    const alpha = opacity / 100;
    let dynamicStyle = document.getElementById("dynamic-bg-style");
    if (!dynamicStyle) {
      dynamicStyle = document.createElement("style");
      dynamicStyle.id = "dynamic-bg-style";
      document.head.appendChild(dynamicStyle);
    }
    dynamicStyle.textContent = `body::before { background-color: rgba(0,0,0,${alpha}); }`;
    currentBg = { preset, opacity };
    localStorage.setItem("bgSettings", JSON.stringify(currentBg));
  }

  function loadBgSettings() {
    const saved = localStorage.getItem("bgSettings");
    if (saved) {
      currentBg = JSON.parse(saved);
      if (bgSelect) bgSelect.value = currentBg.preset;
      if (overlayOpacity) overlayOpacity.value = currentBg.opacity;
      if (opacityValue) opacityValue.textContent = `${currentBg.opacity}%`;
      applyBackground(currentBg.preset, currentBg.opacity);
    } else {
      applyBackground("default", 50);
    }
  }

  function openBgModal() {
    if (bgModal) bgModal.style.display = "flex";
  }
  function closeBgModal() {
    if (bgModal) bgModal.style.display = "none";
  }

  function saveAndApplyBackground() {
    const preset = bgSelect ? bgSelect.value : "default";
    const opacity = overlayOpacity ? parseInt(overlayOpacity.value) : 50;
    applyBackground(preset, opacity);
    closeBgModal();
  }

  if (bgSettingsBtn) bgSettingsBtn.addEventListener("click", openBgModal);
  if (closeModal) closeModal.addEventListener("click", closeBgModal);
  if (modalBackdrop) modalBackdrop.addEventListener("click", closeBgModal);
  if (applyBgBtn) applyBgBtn.addEventListener("click", saveAndApplyBackground);
  if (overlayOpacity) {
    overlayOpacity.addEventListener("input", () => {
      if (opacityValue) opacityValue.textContent = `${overlayOpacity.value}%`;
    });
  }

  loadBgSettings();

  // ── Views ─────────────────────────────────────────────
  function showAddForm() {
    urlInputContainer.style.display = "block";
    savedUrlsContainer.style.display = "none";
    if (instanceName) instanceName.value = "";
    if (instanceUrl) instanceUrl.value = "";
    if (errorDiv) errorDiv.textContent = "";
    setTimeout(() => instanceName?.focus(), 50);
  }

  function showSavedList() {
    urlInputContainer.style.display = "none";
    savedUrlsContainer.style.display = "block";
  }

  // ── Storage ───────────────────────────────────────────
  function loadInstances() {
    const data = localStorage.getItem("jiraInstances");
    return data ? JSON.parse(data) : [];
  }

  function saveInstances(instances) {
    localStorage.setItem("jiraInstances", JSON.stringify(instances));
  }

  // ── Render ────────────────────────────────────────────
  function renderInstances() {
    if (!instancesList) return;
    instancesList.innerHTML = "";
    const instances = loadInstances();

    if (instances.length === 0) {
      instancesList.innerHTML = `
        <div class="empty-state">
          <div class="empty-icon">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect width="20" height="14" x="2" y="7" rx="2"/><path d="M16 21V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16"/></svg>
          </div>
          <p>No instances yet</p>
          <span>Add your first Jira workspace above</span>
        </div>`;
      return;
    }

    instances.forEach((instance, index) => {
      let domain = instance.url;
      try {
        domain = new URL(instance.url).hostname;
      } catch (_) {}

      // Initials from name (up to 2 chars)
      const initials = instance.name
        .split(/\s+/)
        .map((w) => w[0])
        .join("")
        .slice(0, 2)
        .toUpperCase();

      const row = document.createElement("div");
      row.className = "instance-row";
      row.setAttribute("role", "button");
      row.setAttribute("tabindex", "0");
      row.setAttribute("aria-label", `Open ${instance.name}`);

      row.innerHTML = `
        <div class="instance-icon" aria-hidden="true">${escapeHtml(
          initials
        )}</div>
        <div class="instance-info">
          <div class="instance-name">${escapeHtml(instance.name)}</div>
          <div class="instance-domain">${escapeHtml(domain)}</div>
        </div>
        <button class="delete-btn" aria-label="Delete ${escapeHtml(
          instance.name
        )}" title="Delete">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 6h18M19 6l-1 14H6L5 6M9 6V4h6v2"/></svg>
        </button>`;

      row.addEventListener("click", (e) => {
        if (!e.target.closest(".delete-btn")) {
          openWebsiteInNewWindow(instance.url);
        }
      });

      row.addEventListener("keydown", (e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          openWebsiteInNewWindow(instance.url);
        }
      });

      const deleteBtn = row.querySelector(".delete-btn");
      deleteBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        showDeleteConfirmation(instance, index);
      });

      instancesList.appendChild(row);
    });
  }

  function escapeHtml(str) {
    const div = document.createElement("div");
    div.textContent = str;
    return div.innerHTML;
  }

  // ── Delete Confirmation ───────────────────────────────
  function showDeleteConfirmation(instance, index) {
    const overlay = document.createElement("div");
    overlay.className = "confirm-modal";
    overlay.style.background = "rgba(0,0,0,0.6)";
    overlay.style.backdropFilter = "blur(4px)";
    overlay.style.webkitBackdropFilter = "blur(4px)";

    overlay.innerHTML = `
      <div class="confirm-panel">
        <h4>Delete instance?</h4>
        <p>"${escapeHtml(
          instance.name
        )}" will be removed. This cannot be undone.</p>
        <div class="confirm-actions">
          <button class="btn-ghost" id="cfmCancel">Cancel</button>
          <button class="btn-danger" id="cfmDelete">Delete</button>
        </div>
      </div>`;

    document.body.appendChild(overlay);

    overlay.querySelector("#cfmCancel").addEventListener("click", () => {
      document.body.removeChild(overlay);
    });

    overlay.querySelector("#cfmDelete").addEventListener("click", () => {
      document.body.removeChild(overlay);
      const instances = loadInstances();
      instances.splice(index, 1);
      saveInstances(instances);
      renderInstances();
    });

    overlay.addEventListener("click", (e) => {
      if (e.target === overlay) document.body.removeChild(overlay);
    });
  }

  // ── Validation ────────────────────────────────────────
  function isValidJiraUrl(url) {
    try {
      const parsed = new URL(url);
      const host = parsed.hostname.toLowerCase();

      if (parsed.protocol !== "https:" && parsed.protocol !== "http:") {
        return {
          valid: false,
          message: "URL must start with http:// or https://",
        };
      }

      const isValid = host.endsWith(".atlassian.net") || host.includes("jira");

      return {
        valid: isValid,
        message: isValid
          ? "Valid Jira URL"
          : "URL does not appear to be a Jira instance. Domain must contain 'jira' or end with '.atlassian.net'",
      };
    } catch {
      return {
        valid: false,
        message: "Invalid URL format. Include http:// or https://",
      };
    }
  }

  function addInstance() {
    const name = instanceName?.value.trim() ?? "";
    const url = instanceUrl?.value.trim() ?? "";

    if (!name || !url) {
      if (errorDiv) errorDiv.textContent = "Please fill in all fields.";
      return;
    }

    const validation = isValidJiraUrl(url);
    if (!validation.valid) {
      if (errorDiv) errorDiv.textContent = validation.message;
      return;
    }

    const instances = loadInstances();
    instances.push({ name, url });
    saveInstances(instances);
    renderInstances();
    showSavedList();
    if (instanceName) instanceName.value = "";
    if (instanceUrl) instanceUrl.value = "";
    if (errorDiv) errorDiv.textContent = "";
  }

  // ── Event Listeners ───────────────────────────────────
  if (saveBtn) saveBtn.addEventListener("click", addInstance);
  if (cancelBtn) cancelBtn.addEventListener("click", showSavedList);
  if (addNewBtn) addNewBtn.addEventListener("click", showAddForm);

  // Enter key in URL field submits
  if (instanceUrl) {
    instanceUrl.addEventListener("keydown", (e) => {
      if (e.key === "Enter") addInstance();
    });
  }

  // ── Version ───────────────────────────────────────────
  if (versionApp) {
    if (isTauri) {
      getVersion()
        .then((v) => {
          versionApp.textContent = `v${v}`;
        })
        .catch(() => {
          versionApp.textContent = "";
        });
    } else {
      versionApp.textContent = "v0.1.3-dev";
    }
  }

  // ── External Links ────────────────────────────────────
  [githubLink, maintainerLink].forEach((link, i) => {
    if (!link) return;
    const urls = [
      "https://github.com/cas8398/jira-desktop-unofficial",
      "https://github.com/cas8398",
    ];
    link.addEventListener("click", (e) => {
      e.preventDefault();
      if (isTauri) openUrl(urls[i]);
      else window.open(urls[i], "_blank");
    });
  });

  // ── Init ──────────────────────────────────────────────
  renderInstances();
  showSavedList();
});
