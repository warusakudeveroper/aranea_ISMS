/**
 * IS06S 統合PIN Control Dashboard - Backend Server
 *
 * ESP32デバイスへのAPIプロキシ + 静的ファイル配信
 * Port: 3001
 *
 * 排他制御: キューではなく「操作中ガード」方式
 * - 同一デバイスへのコマンド(POST)実行中は後続コマンドを即reject (busy)
 * - ポーリング(GET /api/devices)はコマンド中デバイスをキャッシュで返す
 */
const express = require('express');
const http = require('http');
const path = require('path');

const app = express();
const PORT = 3001;

// デバイスIP範囲
const DEVICE_START = 101;
const DEVICE_END = 109;
const SUBNET = '192.168.77';

// デバイスキャッシュ: 一時的な応答失敗でoffline扱いしない
// key=ip, value={ data, failCount }
const deviceCache = {};
const FAIL_THRESHOLD = 3;

// コマンド実行中フラグ: key=ip, value=true/false
const deviceBusy = {};

app.use(express.json());
app.use(express.static(path.join(__dirname, 'public')));

// ============================================================
// ヘルパー: ESP32へHTTPリクエスト
// ============================================================
function fetchDevice(ip, apiPath, method = 'GET', body = null) {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: ip,
      port: 80,
      path: apiPath,
      method: method,
      timeout: 4000,
      headers: {}
    };
    if (body) {
      const data = JSON.stringify(body);
      options.headers['Content-Type'] = 'application/json';
      options.headers['Content-Length'] = Buffer.byteLength(data);
    }

    const req = http.request(options, (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        try {
          resolve({ ok: true, status: res.statusCode, data: JSON.parse(data) });
        } catch {
          resolve({ ok: false, status: res.statusCode, error: 'Invalid JSON', raw: data });
        }
      });
    });
    req.on('timeout', () => { req.destroy(); reject(new Error('timeout')); });
    req.on('error', (e) => reject(e));
    if (body) req.write(JSON.stringify(body));
    req.end();
  });
}

// ============================================================
// busyガード付きコマンド送信
// コマンド実行中なら即座に busy応答を返す（キューしない）
// ============================================================
async function guardedCommand(ip, apiPath, method, body) {
  if (deviceBusy[ip]) {
    return { ok: false, busy: true, message: '操作中です。完了までお待ちください' };
  }
  deviceBusy[ip] = true;
  try {
    const result = await fetchDevice(ip, apiPath, method, body);
    return result;
  } finally {
    deviceBusy[ip] = false;
  }
}

// ============================================================
// GET /api/devices - 全デバイスのステータス取得
// ============================================================
app.get('/api/devices', async (req, res) => {
  const devices = [];
  const promises = [];

  for (let i = DEVICE_START; i <= DEVICE_END; i++) {
    const ip = `${SUBNET}.${i}`;

    // コマンド実行中のデバイスはキャッシュを返す（ESP32に負荷をかけない）
    if (deviceBusy[ip]) {
      const cached = deviceCache[ip];
      if (cached) {
        devices.push({ ...cached.data, busy: true });
      } else {
        devices.push({ ip, online: false, busy: true });
      }
      continue;
    }

    promises.push(
      Promise.all([
        fetchDevice(ip, '/api/status').catch(() => null),
        fetchDevice(ip, '/api/settings').catch(() => null)
      ]).then(([statusResult, settingsResult]) => {
          if (statusResult?.ok && statusResult.data.ok) {
            const d = statusResult.data;
            const s = settingsResult?.data?.settings || {};
            // settings取得失敗時はキャッシュからrid/deviceNameを引き継ぐ
            const prev = deviceCache[ip]?.data || {};
            const devData = {
              ip,
              online: true,
              hostname: d.device?.hostname || '?',
              mac: d.device?.mac || '?',
              lacisId: d.device?.lacisId || '?',
              cic: d.device?.cic || '?',
              rid: s.rid || prev.rid || '',
              deviceName: s.device_name || prev.deviceName || '',
              uptime: d.system?.uptimeHuman || '?',
              heap: d.system?.heap || 0,
              rssi: d.network?.rssi || 0,
              chipTemp: d.system?.chipTemp || 0,
              firmware: d.firmware?.version || '?',
              uiVersion: d.firmware?.uiVersion || '?',
              pins: d.typeSpecific?.pins || []
            };
            deviceCache[ip] = { data: devData, failCount: 0 };
            devices.push(devData);
          } else {
            handleDeviceFail(ip, devices);
          }
        })
        .catch(() => {
          handleDeviceFail(ip, devices);
        })
    );
  }

  await Promise.all(promises);
  devices.sort((a, b) => {
    const ridCmp = (a.rid || '').localeCompare(b.rid || '');
    if (ridCmp !== 0) return ridCmp;
    const nameCmp = (a.deviceName || a.hostname || '').localeCompare(b.deviceName || b.hostname || '');
    if (nameCmp !== 0) return nameCmp;
    return a.ip.localeCompare(b.ip);
  });
  res.json({ ok: true, devices, timestamp: new Date().toISOString() });
});

/**
 * デバイス応答失敗時の処理
 */
function handleDeviceFail(ip, devices) {
  const cached = deviceCache[ip];
  if (cached) {
    cached.failCount++;
    if (cached.failCount < FAIL_THRESHOLD) {
      devices.push({ ...cached.data, stale: true });
      return;
    }
  }
  devices.push({ ip, online: false, error: 'Unreachable' });
}

// ============================================================
// GET /api/device/:ip/status - 個別デバイスステータス
// ============================================================
app.get('/api/device/:ip/status', async (req, res) => {
  try {
    const result = await fetchDevice(req.params.ip, '/api/status');
    res.json(result.data);
  } catch (e) {
    res.status(502).json({ ok: false, error: e.message });
  }
});

// ============================================================
// GET /api/device/:ip/pins - PIN状態取得
// ============================================================
app.get('/api/device/:ip/pins', async (req, res) => {
  try {
    const result = await fetchDevice(req.params.ip, '/api/pin/all');
    res.json(result.data);
  } catch (e) {
    res.status(502).json({ ok: false, error: e.message });
  }
});

// ============================================================
// POST /api/device/:ip/pin/:ch/toggle - PINトグル
// ============================================================
app.post('/api/device/:ip/pin/:ch/toggle', async (req, res) => {
  const result = await guardedCommand(req.params.ip, `/api/pin/${req.params.ch}/toggle`, 'POST', {});
  if (result.busy) return res.status(409).json(result);
  if (result.ok) return res.json(result.data);
  res.status(502).json({ ok: false, error: result.error || 'failed' });
});

// ============================================================
// POST /api/device/:ip/pin/:ch/state - PIN状態設定
// ============================================================
app.post('/api/device/:ip/pin/:ch/state', async (req, res) => {
  const result = await guardedCommand(req.params.ip, `/api/pin/${req.params.ch}/state`, 'POST', req.body);
  if (result.busy) return res.status(409).json(result);
  if (result.ok) return res.json(result.data);
  res.status(502).json({ ok: false, error: result.error || 'failed' });
});

// ============================================================
// POST /api/device/:ip/pin/:ch/setting - PIN設定変更
// ============================================================
app.post('/api/device/:ip/pin/:ch/setting', async (req, res) => {
  const result = await guardedCommand(req.params.ip, `/api/pin/${req.params.ch}/setting`, 'POST', req.body);
  if (result.busy) return res.status(409).json(result);
  if (result.ok) return res.json(result.data);
  res.status(502).json({ ok: false, error: result.error || 'failed' });
});

// ============================================================
// POST /api/device/:ip/reboot - デバイス再起動
// ============================================================
app.post('/api/device/:ip/reboot', async (req, res) => {
  const result = await guardedCommand(req.params.ip, '/api/system/reboot', 'POST', {});
  if (result.busy) return res.status(409).json(result);
  if (result.ok) return res.json(result.data);
  res.status(502).json({ ok: false, error: result.error || 'failed' });
});

// ============================================================
// Start
// ============================================================
app.listen(PORT, '0.0.0.0', () => {
  console.log(`IS06S Dashboard running on http://0.0.0.0:${PORT}`);
  console.log(`Monitoring ${SUBNET}.${DEVICE_START} - ${SUBNET}.${DEVICE_END}`);
});
