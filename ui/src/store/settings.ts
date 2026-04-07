import { create } from 'zustand'
import { persist } from 'zustand/middleware'

export type Theme = 'light' | 'dark'
export type Role = 'admin' | 'editor' | 'viewer' | null

interface SettingsState {
  serverUrl: string
  apiKey: string
  theme: Theme
  role: Role
  setServerUrl: (url: string) => void
  setApiKey: (key: string) => void
  setTheme: (theme: Theme) => void
  setRole: (role: Role) => void
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      serverUrl: 'http://localhost:8080',
      apiKey: '',
      theme: 'light',
      role: null,
      setServerUrl: (serverUrl) => set({ serverUrl }),
      setApiKey: (apiKey) => set({ apiKey }),
      setTheme: (theme) => set({ theme }),
      setRole: (role) => set({ role }),
    }),
    { name: 'axiom-settings' },
  ),
)
