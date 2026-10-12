import { post } from './api.js';

export function bindRatingControls({ outEl, onDone }) {
  const btn = document.getElementById('btn-rate');
  if (!btn) return;
  btn.addEventListener('click', async () => {
    outEl.textContent = '…';
    try {
      const body = {
        receipt_id: document.getElementById('did').value,
        score: Number(document.getElementById('score').value),
        comment: '',
      };
      outEl.textContent = JSON.stringify(await post('/api/rate', body), null, 2);
      onDone();
    } catch (e) {
      outEl.textContent = String(e.message || e);
    }
  });
}
