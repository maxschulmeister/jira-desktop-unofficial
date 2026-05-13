# Changelog

All notable changes to this project will be documented in this file.

## [0.1.3] - 2026-06-13

### ✨ New Features

- **Customizable Backgrounds**  
  Choose from 5 beautiful background images from Pexels to personalize your app experience.

- **Dynamic Window Titles**  
  Window titles now update automatically based on the Jira page you're viewing (e.g., "Dashboard - JDU", "Project Settings - JDU").

### 🛠️ Improvements

- **Modern UI Redesign**  
  Completely redesigned interface with a fresh, modern look and feel.

- **Better Jira URL Validation**  
  Fixed issue where valid Jira URLs like `https://domain.atlassian.net/` were being rejected. Now accepts root paths and all valid Jira domains.

- **Improved Domain Detection**  
  Simplified and more reliable Jira Cloud vs. Server detection logic.

- **Window Title Format**  
  Standardized window title format to "Page Name [JDU]" for better identification.

### 🐛 Bug Fixes

- Fixed URL validation rejecting valid Jira instances with trailing slashes.
- Resolved duplicate window title update spam.

---

## [0.1.2] - 2025-06-18

### ✨ New Features

- **Custom URL Support**  
  Connect to any Jira instance — cloud-hosted or self-managed. No more hardcoded endpoints.

- **Multiple Windows**  
  Open multiple Jira tabs in separate native windows for seamless multitasking.

- **Minimal UI**  
  A clean, distraction-free interface. Just Jira — no browser chrome, no clutter.

### 🛠️ Improvements

- **Enhanced Stability & Error Handling**  
  More reliable connections and graceful error recovery. Clear feedback when something goes wrong.

### 🎯 Upgrade Notes

- No breaking changes in this release.
- Existing configurations remain compatible.
