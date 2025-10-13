module.exports = function(eleventyConfig) {
  const fs = require('fs');
  const path = require('path');

  // Shortcode to inline raw file contents (e.g., your existing HTML pages)
  eleventyConfig.addShortcode('rawfile', function(filePath) {
    try {
      const abs = path.resolve(process.cwd(), filePath);
      return fs.readFileSync(abs, 'utf8');
    } catch (e) {
      return `<p style="color:#f66">Missing file: ${filePath}</p>`;
    }
  });

  // Passthrough copy for any static assets you might add under docs_src/static
  eleventyConfig.addPassthroughCopy({ 'docs_src/static': '.' });

  return {
    dir: {
      input: 'docs_src',
      includes: '_includes',
      output: 'docs'
    },
    htmlTemplateEngine: 'njk',
    markdownTemplateEngine: 'njk'
  };
};
