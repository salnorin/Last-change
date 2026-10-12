/** Thin client for the local lastchange node HTTP/JSON API. */
export async function get(path) {
  const r = await fetch(path);
  if (!r.ok) throw new Error(await r.text());
  return r.json();
}

export async function post(path, body) {
  const r = await fetch(path, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });
  const text = await r.text();
  let data;
  try { data = JSON.parse(text); } catch { data = { raw: text }; }
  if (!r.ok) throw new Error(typeof data === 'object' && data.raw ? data.raw : text);
  return data;
}

export function connectEvents(onMessage) {
  const proto = location.protocol === 'https:' ? 'wss://' : 'ws://';
  const ws = new WebSocket(proto + location.host + '/ws');
  ws.onmessage = () => onMessage();
  ws.onclose = () => setTimeout(() => connectEvents(onMessage), 2000);
  return ws;
}
