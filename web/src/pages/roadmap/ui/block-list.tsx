import { Trash2 } from 'lucide-react'
import { cn } from '@/shared/lib'
import { NativeSelect } from '@/shared/ui'
import {
  dayMinutes,
  formatHours,
  formatTime,
  replaced,
  snapMinutes,
  toggledDay,
  weekdays,
  weeklyHours,
  type Block,
} from '../model/week-grid'

const halfHours = Array.from({ length: dayMinutes / snapMinutes + 1 }, (_, index) => index * snapMinutes)

function TimeSelect({ label, value, choices, onChange }: { label: string; value: number; choices: number[]; onChange: (minute: number) => void }) {
  const options = choices.includes(value) ? choices : [...choices, value].sort((a, b) => a - b)
  return (
    <NativeSelect aria-label={label} value={value} onChange={(event) => onChange(Number(event.target.value))} className="h-7 w-24 font-mono text-xs">
      {options.map((minute) => (
        <option key={minute} value={minute}>
          {formatTime(minute)}
        </option>
      ))}
    </NativeSelect>
  )
}

export function BlockList({
  blocks,
  overlapping,
  refused,
  onChange,
}: {
  blocks: Block[]
  overlapping: Set<number>
  refused: Set<number>
  onChange: (blocks: Block[]) => void
}) {
  if (blocks.length === 0) {
    return null
  }
  return (
    <ul aria-label="Blocks" className="space-y-1.5">
      {blocks.map((block, index) => (
        <li
          key={index}
          data-overlap={overlapping.has(index) || undefined}
          data-refused={refused.has(index) || undefined}
          className={cn(
            'flex flex-wrap items-center gap-2 rounded-md border border-border px-2 py-1',
            overlapping.has(index) && 'border-amber-500 border-dashed',
            refused.has(index) && 'border-2 border-solid border-red-600 bg-red-500/15',
          )}
        >
          <div role="group" aria-label={`Days of block ${index + 1}`} className="inline-flex overflow-hidden rounded-md border border-border">
            {weekdays.map((day) => (
              <button
                key={day}
                type="button"
                aria-pressed={block.days.includes(day)}
                onClick={() => onChange(replaced(blocks, index, toggledDay(block, day)))}
                className={cn(
                  'w-8 py-0.5 text-[11px] capitalize text-muted-foreground hover:bg-muted',
                  block.days.includes(day) && 'bg-primary text-primary-foreground hover:bg-primary',
                )}
              >
                {day.slice(0, 2)}
              </button>
            ))}
          </div>
          <TimeSelect
            label={`Start of block ${index + 1}`}
            value={block.from}
            choices={halfHours.filter((minute) => minute < block.to)}
            onChange={(from) => onChange(replaced(blocks, index, { ...block, from }))}
          />
          <span className="text-xs text-muted-foreground">to</span>
          <TimeSelect
            label={`End of block ${index + 1}`}
            value={block.to}
            choices={halfHours.filter((minute) => minute > block.from)}
            onChange={(to) => onChange(replaced(blocks, index, { ...block, to }))}
          />
          <span className="font-mono text-xs text-muted-foreground">{formatHours(weeklyHours([block]))} h / week</span>
          <button
            type="button"
            aria-label={`Remove block ${index + 1}`}
            onClick={() => onChange(replaced(blocks, index, null))}
            className="ml-auto rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
          >
            <Trash2 className="size-3.5" aria-hidden />
          </button>
        </li>
      ))}
    </ul>
  )
}
