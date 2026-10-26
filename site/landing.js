const menu = document.querySelector('.menu-toggle');
menu.addEventListener('click', () => {
  const open = menu.getAttribute('aria-expanded') !== 'true';
  menu.setAttribute('aria-expanded', String(open));
  document.querySelector('.navigation').classList.toggle('is-open', open);
});
document.querySelectorAll('#nav-links a').forEach(link => link.addEventListener('click', () => {
  menu.setAttribute('aria-expanded', 'false');
  document.querySelector('.navigation').classList.remove('is-open');
}));
document.addEventListener('keydown', event => {
  if (event.key === 'Escape') { menu.setAttribute('aria-expanded', 'false'); document.querySelector('.navigation').classList.remove('is-open'); }
});
// Preserve links to the earlier documentation site without taking over landing anchors.
if (/^#\//.test(location.hash)) location.replace(`docs/${location.hash}`);
fetch('docs/compat.json').then(response => { if (!response.ok) throw new Error(response.status); return response.json(); }).then(data => {
  for (const key of ['implemented', 'verified']) {
    const total = data.subsystems.reduce((sum, item) => sum + item.weight * item[key], 0);
    document.getElementById(key).textContent = `≈${total.toFixed(1)}%`;
  }
}).catch(() => { document.getElementById('implemented').textContent = 'See docs'; document.getElementById('verified').textContent = 'See docs'; });
