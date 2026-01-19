import { useState } from 'react'

interface SystemTabProps {
  showToast: (message: string, type: 'success' | 'error' | 'info') => void
}

function SystemTab({ showToast }: SystemTabProps) {
  const [rebooting, setRebooting] = useState(false)

  const handleReboot = async () => {
    if (!confirm('Are you sure you want to reboot the system?')) {
      return
    }

    setRebooting(true)
    try {
      const res = await fetch('/api/system/reboot', { method: 'POST' })
      const data = await res.json()

      if (data.ok) {
        showToast('Rebooting... Page will reload in 30 seconds', 'info')
        setTimeout(() => {
          window.location.reload()
        }, 30000)
      } else {
        showToast('Reboot command failed', 'error')
        setRebooting(false)
      }
    } catch (err) {
      showToast('Reboot request failed', 'error')
      setRebooting(false)
    }
  }

  const handleRestartService = async () => {
    if (!confirm('Restart the IS21 inference service?')) {
      return
    }

    try {
      const res = await fetch('/api/system/restart-service', { method: 'POST' })
      const data = await res.json()

      if (data.ok) {
        showToast('Service restarting... Page will reload in 10 seconds', 'info')
        setTimeout(() => {
          window.location.reload()
        }, 10000)
      } else {
        showToast('Service restart failed', 'error')
      }
    } catch (err) {
      showToast('Service restart request failed', 'error')
    }
  }

  return (
    <div>
      {/* System Controls */}
      <div className="card">
        <div className="card-title">System Controls</div>

        <div style={{ display: 'flex', gap: '12px', flexWrap: 'wrap' }}>
          <button
            className="btn btn-secondary"
            onClick={handleRestartService}
            disabled={rebooting}
          >
            Restart Service
          </button>

          <button
            className="btn btn-danger"
            onClick={handleReboot}
            disabled={rebooting}
          >
            {rebooting ? 'Rebooting...' : 'Reboot System'}
          </button>
        </div>
      </div>

      {/* System Info */}
      <div className="card">
        <div className="card-title">System Information</div>
        <p style={{ color: 'var(--text-muted)', fontSize: '0.875rem' }}>
          IS21 runs as a systemd service (is21-infer.service).
        </p>
        <pre style={{
          background: 'var(--bg-secondary)',
          padding: '12px',
          borderRadius: '6px',
          fontSize: '0.75rem',
          overflow: 'auto',
          marginTop: '12px'
        }}>
{`# Service commands (SSH)
sudo systemctl status is21-infer
sudo systemctl restart is21-infer
sudo journalctl -u is21-infer -f`}
        </pre>
      </div>

      {/* Logs */}
      <div className="card">
        <div className="card-title">Recent Logs</div>
        <p style={{ color: 'var(--text-muted)', fontSize: '0.875rem', marginBottom: '12px' }}>
          View logs via SSH or use the command below:
        </p>
        <pre style={{
          background: 'var(--bg-secondary)',
          padding: '12px',
          borderRadius: '6px',
          fontSize: '0.75rem',
          overflow: 'auto'
        }}>
{`ssh mijeosadmin@192.168.3.240
sudo journalctl -u is21-infer -n 100`}
        </pre>
      </div>

      {/* Danger Zone */}
      <div className="card" style={{ borderColor: 'var(--danger)' }}>
        <div className="card-title" style={{ color: 'var(--danger)' }}>Danger Zone</div>
        <p style={{ color: 'var(--text-muted)', fontSize: '0.875rem', marginBottom: '12px' }}>
          These actions cannot be undone. Use with caution.
        </p>
        <div style={{ display: 'flex', gap: '12px', flexWrap: 'wrap' }}>
          <button
            className="btn btn-danger"
            onClick={() => {
              if (confirm('This will clear all local settings. Continue?')) {
                showToast('Factory reset not implemented', 'info')
              }
            }}
          >
            Factory Reset
          </button>
        </div>
      </div>
    </div>
  )
}

export default SystemTab
