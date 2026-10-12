import { connectEvents } from './api.js';
import { renderIdentity, bindLoginPanel } from './keys.js';
import { loadListings, bindCreateForm } from './listings.js';
import { bindDealControls } from './deals.js';
import { bindRatingControls } from './ratings.js';

function refresh() {
  loadListings({
    q: document.getElementById('q').value,
    kind: document.getElementById('fk').value,
    listEl: document.getElementById('list'),
  }).catch((e) => {
    document.getElementById('list').innerHTML = `<p class="muted">${e.message}</p>`;
  });
}

function boot() {
  renderIdentity(document.getElementById('me'));
  bindLoginPanel(document.getElementById('login'));
  bindCreateForm(document.getElementById('create'), document.getElementById('st'), refresh);
  bindDealControls({ outEl: document.getElementById('out'), onDone: refresh });
  bindRatingControls({ outEl: document.getElementById('out'), onDone: refresh });
  document.getElementById('q').addEventListener('input', refresh);
  document.getElementById('fk').addEventListener('change', refresh);
  connectEvents(refresh);
  refresh();
}

document.addEventListener('DOMContentLoaded', boot);
