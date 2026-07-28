import init, { create_portfolio, list_portfolio, extract_file, validate_portfolio, is_portfolio } from './public/wasm/portfolio_wasm.js';

// ── State ──
const files = new Map(); // name → { data, size, type }
let openPortfolioData = null;
let openPortfolioFiles = [];

// ── DOM refs ──
const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);

const dropZone = $('#drop-zone');
const fileInput = $('#file-input');
const fileList = $('#file-list');
const fileCount = $('#file-count');
const fileListSection = $('#file-list-section');
const actionsSection = $('#actions-section');
const clearBtn = $('#clear-btn');
const createBtn = $('#create-btn');
const createStatus = $('#create-status');
const convertToggle = $('#convert-toggle');
const reorderBtn = $('#reorder-btn');
const openInput = $('#open-input');
const openBtn = $('#open-btn');
const openName = $('#open-name');
const openFileList = $('#open-file-list');
const openActions = $('#open-actions');
const downloadFileBtn = $('#download-file-btn');
const openStatus = $('#open-status');
const validateInput = $('#validate-input');
const validateBtn = $('#validate-btn');
const validateResults = $('#validate-results');

// ── Init ──
async function main() {
  await init();
  bindEvents();
}

function bindEvents() {
  // Drop zone
  dropZone.addEventListener('click', () => fileInput.click());
  dropZone.addEventListener('dragover', (e) => { e.preventDefault(); dropZone.classList.add('drag-over'); });
  dropZone.addEventListener('dragleave', () => dropZone.classList.remove('drag-over'));
  dropZone.addEventListener('drop', handleDrop);
  $('#pick-files-btn').addEventListener('click', () => fileInput.click());
  fileInput.addEventListener('change', () => addFiles(fileInput.files));
  clearBtn.addEventListener('click', clearAll);
  createBtn.addEventListener('click', createPortfolio);
  reorderBtn.addEventListener('click', sortAZ);

  // Open
  openBtn.addEventListener('click', () => openInput.click());
  openInput.addEventListener('change', handleOpen);
  downloadFileBtn.addEventListener('click', downloadSelected);

  // Validate
  validateBtn.addEventListener('click', () => validateInput.click());
  validateInput.addEventListener('change', handleValidate);
}

// ── File handling ──

function handleDrop(e) {
  e.preventDefault();
  dropZone.classList.remove('drag-over');
  addFiles(e.dataTransfer.files);
}

function addFiles(fileList) {
  for (const f of fileList) {
    const reader = new FileReader();
    reader.onload = () => {
      files.set(f.name, { data: new Uint8Array(reader.result), size: f.size, type: f.type });
      renderFileList();
    };
    reader.readAsArrayBuffer(f);
  }
}

function removeFile(name) {
  files.delete(name);
  renderFileList();
}

function clearAll() {
  files.clear();
  renderFileList();
}

function sortAZ() {
  const sorted = Array.from(files.entries()).sort((a, b) => a[0].localeCompare(b[0]));
  files.clear();
  for (const [k, v] of sorted) files.set(k, v);
  renderFileList();
}

function renderFileList() {
  fileList.innerHTML = '';
  if (files.size === 0) {
    fileListSection.hidden = true;
    actionsSection.hidden = true;
    clearBtn.disabled = true;
    return;
  }
  fileListSection.hidden = false;
  actionsSection.hidden = false;
  clearBtn.disabled = false;
  fileCount.textContent = files.size;

  let i = 0;
  for (const [name, info] of files) {
    const li = document.createElement('li');
    li.className = 'file-item';
    li.draggable = true;
    li.dataset.name = name;
    li.innerHTML = `
      <span class="drag-handle">≡</span>
      <span class="file-name">${esc(name)}</span>
      <span class="file-size">${fmt(info.size)}</span>
      <button class="file-remove" data-name="${esc(name)}" title="Remove">×</button>
    `;
    li.querySelector('.file-remove').addEventListener('click', (e) => {
      e.stopPropagation();
      removeFile(name);
    });
    // Drag reorder
    li.addEventListener('dragstart', (e) => { e.dataTransfer.setData('text/plain', name); li.classList.add('dragging'); });
    li.addEventListener('dragend', () => li.classList.remove('dragging'));
    li.addEventListener('dragover', (e) => e.preventDefault());
    li.addEventListener('drop', (e) => {
      e.preventDefault();
      const from = e.dataTransfer.getData('text/plain');
      const to = name;
      if (from !== to) reorder(from, to);
    });
    fileList.appendChild(li);
    i++;
  }
}

function reorder(from, to) {
  const entries = Array.from(files.entries());
  const fromIdx = entries.findIndex(([n]) => n === from);
  const toIdx = entries.findIndex(([n]) => n === to);
  if (fromIdx === -1 || toIdx === -1) return;
  const [item] = entries.splice(fromIdx, 1);
  entries.splice(toIdx, 0, item);
  files.clear();
  for (const [k, v] of entries) files.set(k, v);
  renderFileList();
}

// ── Create Portfolio ──

async function createPortfolio() {
  createBtn.disabled = true;
  createStatus.textContent = 'Creating...';

  try {
    const fileEntries = [];
    for (const [name, info] of files) {
      fileEntries.push({ name, data: Array.from(info.data) });
    }
    const json = JSON.stringify(fileEntries);
    const pdfBytes = create_portfolio(json, convertToggle.checked);
    
    downloadBytes(pdfBytes, 'portfolio.pdf', 'application/pdf');
    createStatus.textContent = 'Done!';
  } catch (e) {
    createStatus.textContent = 'Error: ' + e;
  } finally {
    createBtn.disabled = false;
  }
}

// ── Open Portfolio ──

function handleOpen() {
  const f = openInput.files[0];
  if (!f) return;
  openName.textContent = f.name;
  const reader = new FileReader();
  reader.onload = async () => {
    openPortfolioData = new Uint8Array(reader.result);
    try {
      const json = list_portfolio(openPortfolioData);
      openPortfolioFiles = JSON.parse(json);
      renderOpenFiles();
    } catch (e) {
      openStatus.textContent = 'Error: ' + e;
    }
  };
  reader.readAsArrayBuffer(f);
}

function renderOpenFiles() {
  openFileList.innerHTML = '';
  openActions.hidden = false;
  for (const f of openPortfolioFiles) {
    const li = document.createElement('li');
    li.className = 'file-item';
    li.innerHTML = `
      <span class="file-name">${esc(f.name)}</span>
      <span class="file-size">${fmt(f.size)}</span>
    `;
    li.addEventListener('click', () => {
      li.classList.toggle('selected');
    });
    openFileList.appendChild(li);
  }
}

function downloadSelected() {
  const selected = $$('.file-item.selected .file-name');
  if (selected.length === 0) {
    openStatus.textContent = 'Select files first';
    return;
  }
  for (const el of selected) {
    const name = el.textContent;
    const data = extract_file(openPortfolioData, name);
    downloadBytes(data, name);
  }
}

// ── Validate ──

function handleValidate() {
  const f = validateInput.files[0];
  if (!f) return;
  const reader = new FileReader();
  reader.onload = () => {
    const data = new Uint8Array(reader.result);
    try {
      const json = validate_portfolio(data);
      const issues = JSON.parse(json);
      renderValidation(issues);
    } catch (e) {
      validateResults.innerHTML = `<li class="warn">Error: ${esc(String(e))}</li>`;
    }
  };
  reader.readAsArrayBuffer(f);
}

function renderValidation(issues) {
  validateResults.innerHTML = '';
  for (const issue of issues) {
    const li = document.createElement('li');
    li.className = issue === 'Portfolio is valid' ? 'ok' : 'warn';
    li.textContent = issue;
    validateResults.appendChild(li);
  }
}

// ── Helpers ──

function downloadBytes(bytes, filename, mime = 'application/octet-stream') {
  const blob = new Blob([bytes], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

function fmt(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function esc(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

main();
