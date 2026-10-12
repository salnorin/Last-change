/**
 * Key-based "login" UI.
 * Today the node holds the keystore; this module shows the node's identity and
 * documents the planned browser keystore (WebCrypto + IndexedDB). Light clients
 * never compute circuit-pow stamps — see docs/CLIENTS.md.
 */
import { get } from './api.js';

export async function renderIdentity(el) {
  try {
    const j = await get('/api/whoami');
    el.textContent = `${j.pseudonym} / ${j.pubkey}`;
    el.title = `ML-DSA-65: ${(j.ml_dsa_65 || '').slice(0, 32)}…`;
  } catch (e) {
    el.textContent = 'node unreachable';
  }
}

export function bindLoginPanel(root) {
  root.innerHTML = `
    <p class="muted">Keys live on your node (<code>lastchange wallet new</code>).
    Browser generate/import (encrypted IndexedDB) is planned; phones and browsers never run PoW.</p>
    <div class="row">
      <button type="button" class="secondary" id="btn-refresh-id">Refresh identity</button>
    </div>`;
  root.querySelector('#btn-refresh-id').addEventListener('click', () => {
    renderIdentity(document.getElementById('me'));
  });
}
