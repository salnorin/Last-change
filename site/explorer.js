const g = await (await fetch('genesis.json')).json();
const h = g.header;
document.getElementById('summary').innerHTML =
  `<p><b>Message:</b> ${g.coinbase.msg.replace(/[<>&]/g, '')}</p>
   <p class="muted">time ${new Date(h.time * 1000).toISOString()} · bits ${h.bits} · nonce ${h.nonce} · value ${g.coinbase.value}</p>`;
document.getElementById('json').textContent = JSON.stringify(g, null, 2);
