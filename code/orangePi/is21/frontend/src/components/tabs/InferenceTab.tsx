import { useState, useEffect } from 'react'

interface InferenceTabProps {
  showToast: (message: string, type: 'success' | 'error' | 'info') => void
}

interface InferenceConfig {
  conf_threshold: number
  nms_threshold: number
  par_enabled: boolean
}

interface InferenceStats {
  yolo: {
    totalCount: number
    successCount: number
    errorCount: number
    avgMs: number
  }
  par: {
    enabled: boolean
    totalCount: number
    successCount: number
    errorCount: number
    avgMs: number
  }
  color: {
    totalCount: number
    avgMs: number
  }
}

function InferenceTab({ showToast }: InferenceTabProps) {
  const [stats, setStats] = useState<InferenceStats | null>(null)
  const [config, setConfig] = useState<InferenceConfig>({
    conf_threshold: 0.33,
    nms_threshold: 0.45,
    par_enabled: true
  })
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    fetchData()
    const interval = setInterval(fetchStats, 5000)
    return () => clearInterval(interval)
  }, [])

  const fetchData = async () => {
    try {
      const [statusRes, configRes] = await Promise.all([
        fetch('/api/status'),
        fetch('/api/config')
      ])
      const statusData = await statusRes.json()
      const configData = await configRes.json()

      setStats(statusData.inference)
      setConfig({
        conf_threshold: configData.conf_threshold || 0.33,
        nms_threshold: configData.nms_threshold || 0.45,
        par_enabled: configData.par_enabled !== false
      })
    } catch (err) {
      console.error('Failed to fetch inference data:', err)
    } finally {
      setLoading(false)
    }
  }

  const fetchStats = async () => {
    try {
      const res = await fetch('/api/status')
      const data = await res.json()
      setStats(data.inference)
    } catch (err) {
      console.error('Failed to fetch stats:', err)
    }
  }

  const handleSaveConfig = async () => {
    try {
      const res = await fetch('/api/inference/config', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(config)
      })
      const data = await res.json()

      if (data.ok) {
        showToast('Inference config saved', 'success')
      } else {
        showToast('Failed to save config', 'error')
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
      {/* Model Status */}
      <div className="card">
        <div className="card-title">Model Status</div>
        <div className="status-grid">
          <div className="status-item">
            <div className="status-label">YOLO Model</div>
            <div className="status-value success">yolov5s-640</div>
          </div>
          <div className="status-item">
            <div className="status-label">PAR Model</div>
            <div className={`status-value ${stats?.par.enabled ? 'success' : 'warning'}`}>
              {stats?.par.enabled ? 'par_resnet50' : 'Disabled'}
            </div>
          </div>
          <div className="status-item">
            <div className="status-label">NPU Status</div>
            <div className="status-value success">Active</div>
          </div>
        </div>
      </div>

      {/* Live Statistics */}
      <div className="card">
        <div className="card-title">Live Statistics</div>
        <div className="status-grid">
          <div className="status-item">
            <div className="status-label">YOLO Total</div>
            <div className="status-value">{stats?.yolo.totalCount.toLocaleString() || 0}</div>
          </div>
          <div className="status-item">
            <div className="status-label">YOLO Avg</div>
            <div className="status-value">{stats?.yolo.avgMs.toFixed(2) || 0} ms</div>
          </div>
          <div className="status-item">
            <div className="status-label">YOLO Errors</div>
            <div className={`status-value ${(stats?.yolo.errorCount || 0) > 0 ? 'danger' : 'success'}`}>
              {stats?.yolo.errorCount || 0}
            </div>
          </div>
          {stats?.par.enabled && (
            <>
              <div className="status-item">
                <div className="status-label">PAR Total</div>
                <div className="status-value">{stats?.par.totalCount.toLocaleString() || 0}</div>
              </div>
              <div className="status-item">
                <div className="status-label">PAR Avg</div>
                <div className="status-value">{stats?.par.avgMs.toFixed(2) || 0} ms</div>
              </div>
            </>
          )}
          <div className="status-item">
            <div className="status-label">Color Analysis</div>
            <div className="status-value">{stats?.color.totalCount.toLocaleString() || 0}</div>
          </div>
        </div>
      </div>

      {/* Inference Config */}
      <div className="card">
        <div className="card-title">Inference Configuration</div>

        <div className="form-group">
          <label className="form-label">
            Confidence Threshold: {config.conf_threshold.toFixed(2)}
          </label>
          <input
            type="range"
            min="0.1"
            max="0.9"
            step="0.01"
            value={config.conf_threshold}
            onChange={e => setConfig(prev => ({ ...prev, conf_threshold: parseFloat(e.target.value) }))}
            style={{ width: '100%' }}
          />
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.75rem', color: 'var(--text-muted)' }}>
            <span>0.1 (More detections)</span>
            <span>0.9 (Fewer, confident)</span>
          </div>
        </div>

        <div className="form-group">
          <label className="form-label">
            NMS Threshold: {config.nms_threshold.toFixed(2)}
          </label>
          <input
            type="range"
            min="0.1"
            max="0.9"
            step="0.01"
            value={config.nms_threshold}
            onChange={e => setConfig(prev => ({ ...prev, nms_threshold: parseFloat(e.target.value) }))}
            style={{ width: '100%' }}
          />
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.75rem', color: 'var(--text-muted)' }}>
            <span>0.1 (Strict overlap)</span>
            <span>0.9 (Allow overlap)</span>
          </div>
        </div>

        <div className="form-group">
          <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
            <input
              type="checkbox"
              checked={config.par_enabled}
              onChange={e => setConfig(prev => ({ ...prev, par_enabled: e.target.checked }))}
            />
            <span className="form-label" style={{ margin: 0 }}>Enable PAR (Person Attribute Recognition)</span>
          </label>
        </div>

        <button className="btn btn-primary" onClick={handleSaveConfig}>
          Save Configuration
        </button>
      </div>
    </div>
  )
}

export default InferenceTab
