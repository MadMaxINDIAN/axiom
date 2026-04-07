import { Outlet, NavLink } from 'react-router-dom'
import {
  ListFilter, PlusCircle, Table2, GitFork, Settings,
  Layers, Activity, Moon, Sun,
} from 'lucide-react'
import clsx from 'clsx'
import { useSettingsStore } from '../store/settings'
import { useHealth } from '../api/hooks'
import ConnectionStatus from './ConnectionStatus'

const navItems = [
  { to: '/rules',    label: 'Rules',     icon: ListFilter },
  { to: '/rulesets', label: 'Rulesets',  icon: Layers },
  { to: '/tables',   label: 'Tables',    icon: Table2 },
  { to: '/flow',     label: 'Flow',      icon: GitFork },
  { to: '/settings', label: 'Settings',  icon: Settings },
]

export default function Layout() {
  const { theme, setTheme } = useSettingsStore()
  useHealth() // keep connection status fresh

  return (
    <div className="flex h-screen overflow-hidden bg-gray-50 dark:bg-gray-950">
      {/* Sidebar */}
      <aside className="flex flex-col w-56 border-r border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 shrink-0">
        {/* Logo */}
        <div className="flex items-center gap-2 px-4 py-4 border-b border-gray-200 dark:border-gray-800">
          <Activity className="w-5 h-5 text-brand-500" />
          <span className="font-semibold text-gray-900 dark:text-white tracking-tight">Axiom</span>
        </div>

        {/* Nav */}
        <nav className="flex-1 py-3 space-y-0.5 px-2">
          {navItems.map(({ to, label, icon: Icon }) => (
            <NavLink
              key={to}
              to={to}
              className={({ isActive }) =>
                clsx(
                  'flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm font-medium transition-colors',
                  isActive
                    ? 'bg-brand-50 text-brand-700 dark:bg-brand-950 dark:text-brand-300'
                    : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 hover:text-gray-900 dark:hover:text-gray-100',
                )
              }
            >
              <Icon className="w-4 h-4 shrink-0" />
              {label}
            </NavLink>
          ))}
        </nav>

        {/* Bottom: connection + theme */}
        <div className="px-3 py-3 border-t border-gray-200 dark:border-gray-800 space-y-2">
          <ConnectionStatus />
          <button
            onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
            className="flex items-center gap-2 text-xs text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 transition-colors w-full px-2"
          >
            {theme === 'dark' ? <Sun className="w-3.5 h-3.5" /> : <Moon className="w-3.5 h-3.5" />}
            {theme === 'dark' ? 'Light mode' : 'Dark mode'}
          </button>
        </div>
      </aside>

      {/* Main */}
      <main className="flex-1 overflow-y-auto">
        <Outlet />
      </main>
    </div>
  )
}
