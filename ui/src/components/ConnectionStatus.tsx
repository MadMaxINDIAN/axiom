import { useHealth } from '../api/hooks'
import { useSettingsStore } from '../store/settings'
import clsx from 'clsx'

export default function ConnectionStatus() {
  const { data, isLoading, isError } = useHealth()
  const { role } = useSettingsStore()

  const status = isLoading ? 'connecting' : isError ? 'error' : 'ok'

  return (
    <div className="flex items-center gap-2 px-2 py-1">
      <span className={clsx(
        'w-2 h-2 rounded-full shrink-0',
        status === 'ok'         && 'bg-green-500',
        status === 'connecting' && 'bg-yellow-400 animate-pulse',
        status === 'error'      && 'bg-red-500',
      )} />
      <span className="text-xs text-gray-500 dark:text-gray-400 truncate">
        {status === 'ok'
          ? role
            ? <>{data?.status} · <span className="capitalize font-medium text-gray-700 dark:text-gray-300">{role}</span></>
            : 'Connected'
          : status === 'error'
          ? 'Disconnected'
          : 'Connecting…'}
      </span>
    </div>
  )
}
