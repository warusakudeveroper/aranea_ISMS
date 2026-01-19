import { useState, useEffect } from 'react'

interface StatusTabProps {
  showToast: (message: string, type: 'success' | 'error' | 'info') => void
}

interface StatusData {
  device: {
    type: string
    name: string
    productType: string
    productCode: string
    firmwareVersion: string
  }
  lacisId: string
  mac: string
  registration: {
    registered: boolean
    cic_code: string
  }
  uptime: string
  uptimeSeconds: number
  health: {
    memoryPercent: number
    cpuLoad: number
    cpuTemp: number
    npuTemp: number
  }
  inference: {
    yolo: {
      totalCount: number
      successCount: number
      errorCount: number
      avgMs: number
    }
    par: {
      enabled: boolean
      totalCount: number
      avgMs: number
    }
  }
}

function StatusTab({ showToast: _showToast }: StatusTabProps) {
  const [status, setStatus] = useState<StatusData | null>(null)
  const [loading, setLoading] = useState(true)
  void _showToast // future use

  useEffect(() => {
    fetchStatus()
    const interval = setInterval(fetchStatus, 10000) // Refresh every 10s
    return () => clearInterval(interval)
  }, [])

  const fetchStatus = async () => {
    try {
      const res = await fetch('/api/status')
      const data = await res.json()
      setStatus(data)
    } catch (err) {
      console.error('Failed to fetch status:', err)
    } finally {
      setLoading(false)
    }
  }

  if (loading) {
    return (
      <div className="loading">
        <div className="spinner"></div>
      </div>
    )
  }

  if (!status) {
    return <div className="card">Failed to load status</div>
  }

  return (
    <div>
      {/* Device Info */}
      <div className="card">
        <div className="card-title">Device Information</div>
        <div className="status-grid">
          <div className="status-item">
            <div className="status-label">Device Type</div>
            <div className="status-value">{status.device.type}</div>
          </div>
          <div className="status-item">
            <div className="status-label">Firmware</div>
            <div className="status-value">{status.device.firmwareVersion}</div>
          </div>
          <div className="status-item">
            <div className="status-label">LacisID</div>
            <div className="status-value" style={{ fontSize: '0.75rem' }}>
              {status.lacisId}
            </div>
          </div>
          <div className="status-item">
            <div className="status-label">MAC Address</div>
            <div className="status-value">{status.mac}</div>
          </div>
          <div className="status-item">
            <div className="status-label">Uptime</div>
            <div className="status-value">{status.uptime}</div>
          </div>
          <div className="status-item">
            <div className="status-label">Registration</div>
            <div className={`status-value ${status.registration.registered ? 'success' : 'warning'}`}>
              {status.registration.registered ? `CIC: ${status.registration.cic_code}` : 'Not Registered'}
            </div>
          </div>
        </div>
      </div>

      {/* System Health */}
      <div className="card">
        <div className="card-title">System Health</div>
        <div className="status-grid">
          <div className="status-item">
            <div className="status-label">CPU Temperature</div>
            <div className={`status-value ${status.health.cpuTemp > 70 ? 'warning' : ''}`}>
              {status.health.cpuTemp.toFixed(1)}°C
            </div>
          </div>
          <div className="status-item">
            <div className="status-label">NPU Temperature</div>
            <div className={`status-value ${status.health.npuTemp > 70 ? 'warning' : ''}`}>
              {status.health.npuTemp.toFixed(1)}°C
            </div>
          </div>
          <div className="status-item">
            <div className="status-label">Memory Usage</div>
            <div className={`status-value ${status.health.memoryPercent > 80 ? 'warning' : ''}`}>
              {status.health.memoryPercent.toFixed(1)}%
            </div>
          </div>
          <div className="status-item">
            <div className="status-label">CPU Load</div>
            <div className="status-value">{status.health.cpuLoad.toFixed(1)}%</div>
          </div>
        </div>
      </div>

      {/* Inference Stats */}
      <div className="card">
        <div className="card-title">Inference Statistics</div>
        <div className="status-grid">
          <div className="status-item">
            <div className="status-label">YOLO Inferences</div>
            <div className="status-value">{status.inference.yolo.totalCount.toLocaleString()}</div>
          </div>
          <div className="status-item">
            <div className="status-label">YOLO Avg Latency</div>
            <div className="status-value">{status.inference.yolo.avgMs.toFixed(2)} ms</div>
          </div>
          <div className="status-item">
            <div className="status-label">PAR Status</div>
            <div className={`status-value ${status.inference.par.enabled ? 'success' : 'warning'}`}>
              {status.inference.par.enabled ? 'Enabled' : 'Disabled'}
            </div>
          </div>
          {status.inference.par.enabled && (
            <>
              <div className="status-item">
                <div className="status-label">PAR Inferences</div>
                <div className="status-value">{status.inference.par.totalCount.toLocaleString()}</div>
              </div>
              <div className="status-item">
                <div className="status-label">PAR Avg Latency</div>
                <div className="status-value">{status.inference.par.avgMs.toFixed(2)} ms</div>
              </div>
            </>
          )}
          <div className="status-item">
            <div className="status-label">Success Rate</div>
            <div className="status-value success">
              {status.inference.yolo.totalCount > 0
                ? ((status.inference.yolo.successCount / status.inference.yolo.totalCount) * 100).toFixed(2)
                : 0}%
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export default StatusTab
