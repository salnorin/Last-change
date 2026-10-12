import { get, post } from './api.js';

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => (
    { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]
  ));
}

export async function loadListings({ q, kind, listEl }) {
  const u = new URLSearchParams();
  if (q) u.set('q', q);
  if (kind) u.set('kind', kind);
  const ls = await get('/api/listings?' + u);
  listEl.innerHTML = ls.length
    ? ls.map((x) => `
      <article class="card">
        <h3>${esc(x.listing.title)}</h3>
        <p class="muted">[${esc(x.listing.kind)}] ${esc(x.listing.category)}</p>
        <p>${esc(x.listing.desc)}</p>
        <p class="muted">Price hint: ${esc(x.listing.price_hint)} · seller ${esc(x.seller_pseudonym.slice(0, 12))}
          · reputation ${x.rep ? x.rep.mean.toFixed(2) + ' (' + x.rep.count + ')' : '—'}</p>
        <p><button type="button" class="secondary pick-id" data-id="${esc(x.id)}">Use id</button>
           <code class="muted">${esc(x.id)}</code></p>
      </article>`).join('')
    : '<p class="muted">No listings yet</p>';
  listEl.querySelectorAll('.pick-id').forEach((btn) => {
    btn.addEventListener('click', () => {
      const did = document.getElementById('did');
      if (did) did.value = btn.dataset.id;
    });
  });
}

export function bindCreateForm(form, statusEl, onDone) {
  form.addEventListener('submit', async (ev) => {
    ev.preventDefault();
    const fd = new FormData(form);
    statusEl.textContent = 'Node is computing anti-spam stamp…';
    try {
      const images = String(fd.get('images') || '')
        .split(',').map((s) => s.trim()).filter(Boolean);
      const res = await post('/api/post', {
        kind: fd.get('kind'),
        title: fd.get('title'),
        desc: fd.get('desc') || '',
        price_hint: fd.get('hint') || '',
        category: fd.get('category') || 'misc',
        images,
      });
      statusEl.textContent = JSON.stringify(res);
      form.reset();
      onDone();
    } catch (e) {
      statusEl.textContent = String(e.message || e);
    }
  });
}
