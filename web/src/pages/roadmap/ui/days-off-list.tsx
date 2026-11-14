import { Plus, X } from 'lucide-react'
import { useState } from 'react'
import { Button, Input } from '@/shared/ui'
import { dayOffEntry, withDayOff } from '../model/calendar-draft'

export function DaysOffList({
  label,
  days,
  editable,
  onChange,
}: {
  label: string
  days: string[]
  editable: boolean
  onChange: (days: string[]) => void
}) {
  const [from, setFrom] = useState('')
  const [to, setTo] = useState('')
  const entry = dayOffEntry(from, to)

  const add = () => {
    if (entry !== null) {
      onChange(withDayOff(days, entry))
      setFrom('')
      setTo('')
    }
  }

  return (
    <div className="space-y-2" role="group" aria-label={label}>
      <p className="text-sm font-medium">{label}</p>
      {days.length === 0 ? (
        <p className="text-xs text-muted-foreground">None</p>
      ) : (
        <ul className="flex flex-wrap gap-1.5">
          {days.map((day) => (
            <li key={day} className="flex items-center gap-1 rounded-full border border-border py-0.5 pr-1 pl-2.5 font-mono text-xs">
              {day.replace('..', ' → ')}
              {editable && (
                <button
                  type="button"
                  aria-label={`Remove ${day}`}
                  onClick={() => onChange(days.filter((entry) => entry !== day))}
                  className="rounded-full p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground"
                >
                  <X className="size-3" aria-hidden />
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
      {editable && (
        <div className="flex flex-wrap items-center gap-2">
          <Input type="date" aria-label={`${label}: first day`} value={from} onChange={(event) => setFrom(event.target.value)} className="h-8 w-40" />
          <span className="text-xs text-muted-foreground">to</span>
          <Input type="date" aria-label={`${label}: last day`} value={to} min={from} onChange={(event) => setTo(event.target.value)} className="h-8 w-40" />
          <Button size="sm" variant="outline" disabled={entry === null} onClick={add}>
            <Plus className="size-4" aria-hidden />
            Add
          </Button>
        </div>
      )}
    </div>
  )
}
