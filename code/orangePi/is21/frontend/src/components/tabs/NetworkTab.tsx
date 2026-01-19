import { useState, useEffect } from 'react'

interface NetworkTabProps {
  showToast: (message: string, type: 'success' | 'error' | 'info') => void
}

interface NetworkInfo {
  hostname: string
  ntp_server: string
  interfaces: {
    name: string
    mac: string
    ipv4: string
    is_up: boolean
  }[]
}

function NetworkTab({ showToast }: NetworkTabProps) {
  const [network, setNetwork] = useState<NetworkInfo | null>(null)
  const [loading, setLoading] = useState(true)
  const [hostname, setHostname] = useState('')
  const [ntpServer, setNtpServer] = useState('pool.ntp.org')

  useEffect(() => {
    fetchNetwork()
  }, [])

  const fetchNetwork = async () => {
    try {
      const [hwRes, configRes] = await Promise.all([
        fetch('/api/hardware'),
        fetch('/api/config')
      ])
      const hwData = await hwRes.json()
      const configData = await configRes.json()

      setNetwork({
        hostname: configData.hostname || `is21-${hwData.network?.[0]?.mac?.replace(/:/g, '').slice(-6) || 'unknown'}`,
        ntp_server: configData.ntp_server || 'pool.ntp.org',
        interfaces: hwData.network || []
      })
      setHostname(configData.hostname || '')
      setNtpServer(configData.ntp_server || 'pool.ntp.org')
    } catch (err) {
      showToast('Failed to fetch network info', 'error')
    } finally {
      setLoading(false)
    }
  }

  const handleSave = async () => {
    try {
      const res = await fetch('/api/network/save', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ hostname, ntp_server: ntpServer })
      })
      const data = await res.json()

      if (data.ok) {
        showToast('Network settings saved', 'success')
      } else {
        showToast('Failed to save settings', 'error')
      }
    } catch (err) {
      showToast('Save request failed', 'error')
    }
  }

  if (loading) {
    return (
      <div className="loading">
        <div className="spinner"></div>
      </div>
    )
  }

  return (
    <div>
      {/* Network Interfaces */}
      <div className="card">
        <div className="card-title">Network Interfaces</div>
        {network?.interfaces.map((iface, idx) => (
          <div key={idx} className="status-item" style={{ marginBottom: '8px' }}>
            <div className="status-grid">
              <div>
                <div className="status-label">{iface.name}</div>
                <div className="status-value">{iface.ipv4 || 'No IP'}</div>
              </div>
              <div>
                <div className="status-label">MAC</div>
                <div className="status-value" style={{ fontSize: '0.75rem' }}>{iface.mac}</div>
              </div>
              <div>
                <div className="status-label">Status</div>
                <div className={`status-value ${iface.is_up ? 'success' : 'danger'}`}>
                  {iface.is_up ? 'Up' : 'Down'}
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Network Settings */}
      <div className="card">
        <div className="card-title">Network Settings</div>

        <div className="form-group">
          <label className="form-label">Hostname</label>
          <input
            type="text"
            className="form-input"
            value={hostname}
            onChange={e => setHostname(e.target.value)}
            placeholder="is21-xxxxxx"
          />
        </div>

        <div className="form-group">
          <label className="form-label">NTP Server</label>
          <input
            type="text"
            className="form-input"
            value={ntpServer}
            onChange={e => setNtpServer(e.target.value)}
            placeholder="pool.ntp.org"
          />
        </div>

        <button className="btn btn-primary" onClick={handleSave}>
          Save
        </button>
      </div>

      {/* Note */}
      <div className="card">
        <div className="card-title">Note</div>
        <p style={{ color: 'var(--text-muted)', fontSize: '0.875rem' }}>
          IS21 runs on Linux. WiFi and IP settings are managed at the OS level.
          Use the operating system tools to configure network interfaces.
        </p>
      </div>
    </div>
  )
}

export default NetworkTab
