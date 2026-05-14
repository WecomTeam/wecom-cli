#!/usr/bin/env node

import fs from 'fs';
import path from 'path';
import crypto from 'crypto';

function usage() {
  console.error('Usage: node skills/wecomcli-doc/scripts/extract-md-images.mjs <markdown-file> [...more]');
  process.exit(1);
}

function mimeToExtension(mime) {
  const normalized = mime.toLowerCase();
  if (normalized === 'jpeg' || normalized === 'jpg') return 'jpg';
  if (normalized === 'svg+xml') return 'svg';
  if (normalized === 'png' || normalized === 'gif' || normalized === 'webp' || normalized === 'bmp' || normalized === 'tiff') {
    return normalized;
  }
  return 'png';
}

function sanitizeName(name) {
  return name
    .replace(/[\\/:*?"<>|]/g, '-')
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
}

function extractImages(markdownPath) {
  const markdownDir = path.dirname(markdownPath);
  const markdownBase = path.basename(markdownPath, path.extname(markdownPath));
  const imageDir = path.join(markdownDir, 'image');
  fs.mkdirSync(imageDir, { recursive: true });

  const source = fs.readFileSync(markdownPath, 'utf8');
  let imageIndex = 0;
  let replacedCount = 0;

  const nextFileName = (mime, buffer) => {
    imageIndex += 1;
    const ext = mimeToExtension(mime);
    const digest = crypto.createHash('sha1').update(buffer).digest('hex').slice(0, 8);
    const base = sanitizeName(markdownBase);
    return `${base}-${String(imageIndex).padStart(3, '0')}-${digest}.${ext}`;
  };

  const replaced = source.replace(
    /!\[([^\]]*)\]\(data:image\/([a-zA-Z0-9.+-]+);base64,([^)]+)\)/g,
    (fullMatch, altText, mime, base64) => {
      const buffer = Buffer.from(base64.replace(/\s+/g, ''), 'base64');
      const fileName = nextFileName(mime, buffer);
      const filePath = path.join(imageDir, fileName);
      fs.writeFileSync(filePath, buffer);
      replacedCount += 1;
      return `![${altText || ''}](image/${fileName})`;
    }
  );

  // 兼容少量导出内容里可能出现的 HTML <img src="data:image/..."> 形式。
  const htmlReplaced = replaced.replace(
    /<img([^>]*?)src="data:image\/([a-zA-Z0-9.+-]+);base64,([^"]+)"([^>]*)>/g,
    (fullMatch, beforeSrc, mime, base64, afterSrc) => {
      const buffer = Buffer.from(base64.replace(/\s+/g, ''), 'base64');
      const fileName = nextFileName(mime, buffer);
      const filePath = path.join(imageDir, fileName);
      fs.writeFileSync(filePath, buffer);
      replacedCount += 1;
      return `<img${beforeSrc}src="image/${fileName}"${afterSrc}>`;
    }
  );

  fs.writeFileSync(markdownPath, htmlReplaced, 'utf8');
  return replacedCount;
}

const files = process.argv.slice(2);
if (files.length === 0) usage();

let total = 0;
for (const file of files) {
  if (!fs.existsSync(file)) {
    console.error(`Skip missing file: ${file}`);
    continue;
  }
  const stat = fs.statSync(file);
  if (!stat.isFile()) {
    console.error(`Skip non-file: ${file}`);
    continue;
  }
  total += extractImages(path.resolve(file));
}

console.log(`Extracted ${total} images`);
