interface ToastProps {
  message: string
  type: 'success' | 'error' | 'info'
}

function Toast({ message, type }: ToastProps) {
  return (
    <div className={`toast ${type}`}>
      {message}
    </div>
  )
}

export default Toast
