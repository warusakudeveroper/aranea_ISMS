// API Base URL - in production, use the IS22 backend port
export const API_BASE_URL = import.meta.env.DEV
  ? "" // In dev mode, Vite proxy handles /api/*
  : "http://192.168.125.246:8080"
