const fs = require('fs');
const path = require('path');

module.exports = () => {
  const root = process.cwd();
  const src = path.resolve(root, 'docs/catalog.json');
  try {
    const raw = fs.readFileSync(src, 'utf8');
    const json = JSON.parse(raw);
    return json;
  } catch (e) {
    return { patterns: [], features: [], achievements: [] };
  }
};
