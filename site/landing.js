/* openXplane landing — gallery, lightbox, navigation */

(function () {
  'use strict';

  /* --- Hamburger --- */
  var ham = document.querySelector('.hamburger');
  var nav = document.getElementById('nav-links');
  if (ham && nav) {
    ham.addEventListener('click', function () {
      var open = ham.getAttribute('aria-expanded') !== 'true';
      ham.setAttribute('aria-expanded', String(open));
      nav.classList.toggle('open', open);
    });
    nav.querySelectorAll('a').forEach(function (a) {
      a.addEventListener('click', function () {
        ham.setAttribute('aria-expanded', 'false');
        nav.classList.remove('open');
      });
    });
    document.addEventListener('keydown', function (e) {
      if (e.key === 'Escape') { ham.setAttribute('aria-expanded', 'false'); nav.classList.remove('open'); }
    });
  }

  /* --- Gallery filters --- */
  var filters = document.querySelectorAll('.filter');
  var items = document.querySelectorAll('.gallery-item');
  filters.forEach(function (btn) {
    btn.addEventListener('click', function () {
      filters.forEach(function (b) { b.classList.remove('active'); });
      btn.classList.add('active');
      var tag = btn.getAttribute('data-filter');
      items.forEach(function (item) {
        if (tag === 'all' || item.getAttribute('data-tags') === tag) {
          item.classList.remove('hidden');
        } else {
          item.classList.add('hidden');
        }
      });
    });
  });

  /* --- Lightbox --- */
  var box = document.createElement('div');
  box.className = 'lightbox';
  box.innerHTML = '<button class="lightbox-close" aria-label="Close">&times;</button><img src="" alt=""><div class="lightbox-caption"></div>';
  document.body.appendChild(box);
  var lbImg = box.querySelector('img');
  var lbCap = box.querySelector('.lightbox-caption');
  var lbClose = box.querySelector('.lightbox-close');

  function openLightbox(src, alt, caption) {
    lbImg.src = src;
    lbImg.alt = alt;
    lbCap.textContent = caption;
    box.classList.add('open');
    document.body.style.overflow = 'hidden';
  }
  function closeLightbox() {
    box.classList.remove('open');
    document.body.style.overflow = '';
  }

  items.forEach(function (fig) {
    fig.addEventListener('click', function () {
      var img = fig.querySelector('img');
      var cap = fig.querySelector('figcaption');
      openLightbox(img.src, img.alt, cap ? cap.textContent : '');
    });
  });
  lbClose.addEventListener('click', closeLightbox);
  box.addEventListener('click', function (e) { if (e.target === box) closeLightbox(); });
  document.addEventListener('keydown', function (e) { if (e.key === 'Escape') closeLightbox(); });

  /* --- Compat stats --- */
  if (/^#\//.test(location.hash)) location.replace('docs/' + location.hash);
  fetch('docs/compat.json')
    .then(function (r) { if (!r.ok) throw new Error(r.status); return r.json(); })
    .then(function (data) {
      ['implemented', 'verified'].forEach(function (key) {
        var el = document.getElementById(key);
        if (!el || !data.subsystems) return;
        var total = data.subsystems.reduce(function (s, i) { return s + i.weight * i[key]; }, 0);
        el.textContent = '≈' + total.toFixed(1) + '%';
      });
    })
    .catch(function () {
      var el = document.getElementById('implemented');
      if (el) el.textContent = 'See docs';
      el = document.getElementById('verified');
      if (el) el.textContent = 'See docs';
    });
})();
