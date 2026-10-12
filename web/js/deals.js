import { post } from './api.js';

async function run(outEl, path, body) {
  outEl.textContent = '…';
  try {
    outEl.textContent = JSON.stringify(await post('/api/' + path, body), null, 2);
  } catch (e) {
    outEl.textContent = String(e.message || e);
  }
}

export function bindDealControls({ outEl, onDone }) {
  const v = (id) => document.getElementById(id).value;
  const actions = {
    'accept': () => run(outEl, 'accept', { listing_id: v('did'), buyer: v('buyer') }),
    'buyer-paid': () => run(outEl, 'step', { accept_id: v('did'), step: 'buyer_paid' }),
    'seller-shipped': () => run(outEl, 'step', { accept_id: v('did'), step: 'seller_shipped' }),
    'buyer-confirmed': () => run(outEl, 'step', { accept_id: v('did'), step: 'buyer_confirmed' }),
    'buyer-disputed': () => run(outEl, 'step', { accept_id: v('did'), step: 'buyer_disputed' }),
    'claim': () => run(outEl, 'claim', { accept_id: v('did'), reason: 'not delivered' }),
    'complete': () => run(outEl, 'complete', { listing_id: v('did'), buyer: v('buyer') }),
  };
  document.querySelectorAll('[data-deal]').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const fn = actions[btn.dataset.deal];
      if (fn) { await fn(); onDone(); }
    });
  });
}
