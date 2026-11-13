import { cn } from '@/shared/lib'
import { levelLetters, levelNames, levels, type Level } from '../model/levels'

export function LevelToggle({ active, onToggle }: { active: readonly Level[]; onToggle: (level: Level) => void }) {
  return (
    <div role="group" aria-label="Levels" className="inline-flex overflow-hidden rounded-md border border-border">
      {levels.map((level) => {
        const pressed = active.includes(level)
        const last = pressed && active.length === 1
        return (
          <button
            key={level}
            type="button"
            aria-pressed={pressed}
            aria-label={levelNames[level]}
            title={last ? `${levelNames[level]}: at least one level stays shown` : levelNames[level]}
            onClick={() => onToggle(level)}
            className={cn(
              'w-8 py-1 font-mono text-xs text-muted-foreground transition-colors duration-(--duration-hover) ease-(--ease-hover) hover:bg-muted',
              pressed && 'bg-primary text-primary-foreground hover:bg-primary',
              last && 'cursor-default',
            )}
          >
            {levelLetters[level]}
          </button>
        )
      })}
    </div>
  )
}
