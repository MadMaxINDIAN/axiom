import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { useSettingsStore } from './store/settings'
import Layout from './components/Layout'
import RulesPage from './pages/Rules'
import RuleNewPage from './pages/RuleNew'
import RuleDetailPage from './pages/RuleDetail'
import RulesetsPage from './pages/Rulesets'
import TablesPage from './pages/Tables'
import FlowPage from './pages/Flow'
import SettingsPage from './pages/Settings'

export default function App() {
  const { theme } = useSettingsStore()

  return (
    <div className={theme === 'dark' ? 'dark' : ''}>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<Layout />}>
            <Route index element={<Navigate to="/rules" replace />} />
            <Route path="rules" element={<RulesPage />} />
            <Route path="rules/new" element={<RuleNewPage />} />
            <Route path="rules/:id" element={<RuleDetailPage />} />
            <Route path="rulesets" element={<RulesetsPage />} />
            <Route path="tables" element={<TablesPage />} />
            <Route path="flow" element={<FlowPage />} />
            <Route path="settings" element={<SettingsPage />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </div>
  )
}
