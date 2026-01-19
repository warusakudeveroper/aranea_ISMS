import { useState, useEffect } from 'react'

interface CloudTabProps {
  showToast: (message: string, type: 'success' | 'error' | 'info') => void
  onRegistrationChange: () => void
}

interface RegistrationInfo {
  registered: boolean
  cic_code: string
  lacis_id: string
  tid: string
  state_endpoint: string
  mqtt_endpoint: string
}

function CloudTab({ showToast, onRegistrationChange }: CloudTabProps) {
  const [loading, setLoading] = useState(true)
  const [activating, setActivating] = useState(false)
  const [registration, setRegistration] = useState<RegistrationInfo | null>(null)

  // Activation form
  const [tid, setTid] = useState('T9999999999999999999')
  const [tenantLacisId, setTenantLacisId] = useState('')
  const [tenantEmail, setTenantEmail] = useState('')
  const [tenantCic, setTenantCic] = useState('')

  useEffect(() => {
    fetchRegistration()
  }, [])

  const fetchRegistration = async () => {
    try {
      const res = await fetch('/api/status')
      const data = await res.json()
      setRegistration(data.registration)
    } catch (err) {
      showToast('Failed to fetch registration status', 'error')
    } finally {
      setLoading(false)
    }
  }

  const handleActivate = async () => {
    if (!tid || !tenantLacisId || !tenantEmail || !tenantCic) {
      showToast('Please fill in all fields', 'error')
      return
    }

    setActivating(true)
    try {
      const res = await fetch('/api/cloud/activate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          tid,
          tenant_primary: {
            lacis_id: tenantLacisId,
            user_id: tenantEmail,
            cic: tenantCic
          }
        })
      })

      const data = await res.json()

      if (data.ok) {
        showToast(`Activated! CIC: ${data.cic_code}`, 'success')
        fetchRegistration()
        onRegistrationChange()
      } else {
        showToast(`Activation failed: ${data.error || 'Unknown error'}`, 'error')
      }
    } catch (err) {
      showToast('Activation request failed', 'error')
    } finally {
      setActivating(false)
    }
  }

  const handleClearRegistration = async () => {
    if (!confirm('Are you sure you want to clear registration? This cannot be undone.')) {
      return
    }

    try {
      const res = await fetch('/api/register', { method: 'DELETE' })
      const data = await res.json()

      if (data.ok) {
        showToast('Registration cleared', 'success')
        fetchRegistration()
        onRegistrationChange()
      } else {
        showToast('Failed to clear registration', 'error')
      }
    } catch (err) {
      showToast('Clear registration request failed', 'error')
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
      {/* Registration Status Card */}
      <div className="card">
        <div className="card-title">Registration Status</div>
        <div className="status-grid">
          <div className="status-item">
            <div className="status-label">Status</div>
            <div className={`status-value ${registration?.registered ? 'success' : 'warning'}`}>
              {registration?.registered ? 'Registered' : 'Not Registered'}
            </div>
          </div>
          {registration?.registered && (
            <>
              <div className="status-item">
                <div className="status-label">CIC</div>
                <div className="status-value">{registration.cic_code}</div>
              </div>
              <div className="status-item">
                <div className="status-label">TID</div>
                <div className="status-value" style={{ fontSize: '0.75rem' }}>
                  {registration.tid}
                </div>
              </div>
            </>
          )}
        </div>
        {registration?.registered && (
          <div style={{ marginTop: '12px' }}>
            <div className="status-item" style={{ marginBottom: '8px' }}>
              <div className="status-label">State Endpoint</div>
              <div className="status-value" style={{ fontSize: '0.75rem', wordBreak: 'break-all' }}>
                {registration.state_endpoint || '-'}
              </div>
            </div>
            <div className="status-item">
              <div className="status-label">MQTT Endpoint</div>
              <div className="status-value" style={{ fontSize: '0.75rem', wordBreak: 'break-all' }}>
                {registration.mqtt_endpoint || '-'}
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Activation Form (only show if not registered) */}
      {!registration?.registered && (
        <div className="card">
          <div className="card-title">Device Activation</div>

          <div className="form-group">
            <label className="form-label">TID (Tenant ID)</label>
            <input
              type="text"
              className="form-input"
              value={tid}
              onChange={e => setTid(e.target.value)}
              placeholder="T9999999999999999999"
            />
          </div>

          <div className="form-group">
            <label className="form-label">Tenant Primary LacisID</label>
            <input
              type="text"
              className="form-input"
              value={tenantLacisId}
              onChange={e => setTenantLacisId(e.target.value)}
              placeholder="14177489787976846466"
            />
          </div>

          <div className="form-group">
            <label className="form-label">Tenant Primary Email</label>
            <input
              type="email"
              className="form-input"
              value={tenantEmail}
              onChange={e => setTenantEmail(e.target.value)}
              placeholder="webadmin@mijeos.com"
            />
          </div>

          <div className="form-group">
            <label className="form-label">Tenant Primary CIC</label>
            <input
              type="text"
              className="form-input"
              value={tenantCic}
              onChange={e => setTenantCic(e.target.value)}
              placeholder="123456"
            />
          </div>

          <button
            className="btn btn-primary"
            onClick={handleActivate}
            disabled={activating}
          >
            {activating ? 'Activating...' : 'Activate'}
          </button>
        </div>
      )}

      {/* Clear Registration (only show if registered) */}
      {registration?.registered && (
        <div className="card">
          <div className="card-title">Danger Zone</div>
          <p style={{ color: 'var(--text-muted)', marginBottom: '12px', fontSize: '0.875rem' }}>
            Clearing registration will remove this device from araneaDeviceGate.
            You will need to re-activate to use cloud features.
          </p>
          <button className="btn btn-danger" onClick={handleClearRegistration}>
            Clear Registration
          </button>
        </div>
      )}
    </div>
  )
}

export default CloudTab
