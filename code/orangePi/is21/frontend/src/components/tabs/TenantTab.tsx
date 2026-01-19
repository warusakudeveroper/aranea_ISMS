import { useState, useEffect } from 'react'

interface TenantTabProps {
  showToast: (message: string, type: 'success' | 'error' | 'info') => void
}

interface TenantInfo {
  tid: string
  fid: string
  device_name: string
  registered: boolean
}

function TenantTab({ showToast }: TenantTabProps) {
  const [tenant, setTenant] = useState<TenantInfo | null>(null)
  const [loading, setLoading] = useState(true)
  const [fid, setFid] = useState('')
  const [deviceName, setDeviceName] = useState('')

  useEffect(() => {
    fetchTenant()
  }, [])

  const fetchTenant = async () => {
    try {
      const [statusRes, configRes] = await Promise.all([
        fetch('/api/status'),
        fetch('/api/config')
      ])
      const statusData = await statusRes.json()
      const configData = await configRes.json()

      const info: TenantInfo = {
        tid: statusData.registration?.tid || configData.tid || '',
        fid: configData.fid || '9966',
        device_name: configData.device_name || 'ParaclateEdge',
        registered: statusData.registration?.registered || false
      }
      setTenant(info)
      setFid(info.fid)
      setDeviceName(info.device_name)
    } catch (err) {
      showToast('Failed to fetch tenant info', 'error')
    } finally {
      setLoading(false)
    }
  }

  const handleSave = async () => {
    try {
      const res = await fetch('/api/tenant/save', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ fid, device_name: deviceName })
      })
      const data = await res.json()

      if (data.ok) {
        showToast('Tenant settings saved', 'success')
        fetchTenant()
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
      {/* Tenant Info */}
      <div className="card">
        <div className="card-title">Tenant Information</div>
        <div className="status-grid">
          <div className="status-item">
            <div className="status-label">TID (Tenant ID)</div>
            <div className="status-value" style={{ fontSize: '0.75rem' }}>
              {tenant?.tid || 'Not set'}
            </div>
          </div>
          <div className="status-item">
            <div className="status-label">Registration</div>
            <div className={`status-value ${tenant?.registered ? 'success' : 'warning'}`}>
              {tenant?.registered ? 'Registered' : 'Not Registered'}
            </div>
          </div>
        </div>
      </div>

      {/* Tenant Settings */}
      <div className="card">
        <div className="card-title">Tenant Settings</div>

        <div className="form-group">
          <label className="form-label">FID (Facility ID)</label>
          <input
            type="text"
            className="form-input"
            value={fid}
            onChange={e => setFid(e.target.value)}
            placeholder="9966"
            maxLength={4}
          />
          <p style={{ color: 'var(--text-muted)', fontSize: '0.75rem', marginTop: '4px' }}>
            IS21 (ParaclateEdge) uses FID 9966 (system facility)
          </p>
        </div>

        <div className="form-group">
          <label className="form-label">Device Name</label>
          <input
            type="text"
            className="form-input"
            value={deviceName}
            onChange={e => setDeviceName(e.target.value)}
            placeholder="ParaclateEdge-01"
          />
        </div>

        <button className="btn btn-primary" onClick={handleSave}>
          Save
        </button>
      </div>

      {/* IS21 Special Note */}
      <div className="card">
        <div className="card-title">About ParaclateEdge</div>
        <p style={{ color: 'var(--text-muted)', fontSize: '0.875rem', marginBottom: '8px' }}>
          IS21 (ParaclateEdge) is a special system device with Permission71.
        </p>
        <ul style={{ color: 'var(--text-muted)', fontSize: '0.875rem', paddingLeft: '20px' }}>
          <li>Belongs to system tenant (T9999999999999999999)</li>
          <li>Has cross-tenant access rights</li>
          <li>Can serve multiple IS22 devices</li>
          <li>FID 9966 is a logical system facility</li>
        </ul>
      </div>
    </div>
  )
}

export default TenantTab
