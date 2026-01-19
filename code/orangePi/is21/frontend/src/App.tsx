import { useState, useEffect } from 'react'
import StatusTab from './components/tabs/StatusTab'
import NetworkTab from './components/tabs/NetworkTab'
import CloudTab from './components/tabs/CloudTab'
import TenantTab from './components/tabs/TenantTab'
import SystemTab from './components/tabs/SystemTab'
import InferenceTab from './components/tabs/InferenceTab'
import Toast from './components/common/Toast'

type TabId = 'status' | 'network' | 'cloud' | 'tenant' | 'system' | 'inference'

interface DeviceInfo {
  type: string
  name: string
  firmwareVersion: string
}

interface ToastMessage {
  id: number
  message: string
  type: 'success' | 'error' | 'info'
}

function App() {
  const [activeTab, setActiveTab] = useState<TabId>('status')
  const [deviceInfo, setDeviceInfo] = useState<DeviceInfo | null>(null)
  const [lacisId, setLacisId] = useState<string>('')
  const [toasts, setToasts] = useState<ToastMessage[]>([])

  useEffect(() => {
    fetchDeviceInfo()
  }, [])

  const fetchDeviceInfo = async () => {
    try {
      const res = await fetch('/api/status')
      const data = await res.json()
      setDeviceInfo(data.device)
      setLacisId(data.lacisId || '')
    } catch (err) {
      console.error('Failed to fetch device info:', err)
    }
  }

  const showToast = (message: string, type: 'success' | 'error' | 'info' = 'info') => {
    const id = Date.now()
    setToasts(prev => [...prev, { id, message, type }])
    setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id))
    }, 3000)
  }

  const tabs: { id: TabId; label: string }[] = [
    { id: 'status', label: 'Status' },
    { id: 'network', label: 'Network' },
    { id: 'cloud', label: 'Cloud' },
    { id: 'tenant', label: 'Tenant' },
    { id: 'system', label: 'System' },
    { id: 'inference', label: 'Inference' },
  ]

  return (
    <div className="container">
      <header className="header">
        <div className="header-title">
          {deviceInfo?.name || 'IS21 ParaclateEdge'}
        </div>
        <div className="header-subtitle">
          {lacisId ? `LacisID: ${lacisId}` : 'Loading...'}
        </div>
      </header>

      <nav className="tabs">
        {tabs.map(tab => (
          <button
            key={tab.id}
            className={`tab ${activeTab === tab.id ? 'active' : ''}`}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </nav>

      <main>
        <div className={`tab-content ${activeTab === 'status' ? 'active' : ''}`}>
          <StatusTab showToast={showToast} />
        </div>
        <div className={`tab-content ${activeTab === 'network' ? 'active' : ''}`}>
          <NetworkTab showToast={showToast} />
        </div>
        <div className={`tab-content ${activeTab === 'cloud' ? 'active' : ''}`}>
          <CloudTab showToast={showToast} onRegistrationChange={fetchDeviceInfo} />
        </div>
        <div className={`tab-content ${activeTab === 'tenant' ? 'active' : ''}`}>
          <TenantTab showToast={showToast} />
        </div>
        <div className={`tab-content ${activeTab === 'system' ? 'active' : ''}`}>
          <SystemTab showToast={showToast} />
        </div>
        <div className={`tab-content ${activeTab === 'inference' ? 'active' : ''}`}>
          <InferenceTab showToast={showToast} />
        </div>
      </main>

      <div className="toast-container">
        {toasts.map(toast => (
          <Toast key={toast.id} message={toast.message} type={toast.type} />
        ))}
      </div>
    </div>
  )
}

export default App
