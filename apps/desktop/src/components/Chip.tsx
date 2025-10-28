import type { ButtonHTMLAttributes } from 'react'

export type ChipVariant = 'status' | 'action'
export type ChipTone = 'primary' | 'idle' | 'info' | 'warning' | 'danger'

interface BaseChipProps {
  label: string
  variant?: ChipVariant
  tone?: ChipTone
  className?: string
}

type InteractiveProps = {
  onClick: ButtonHTMLAttributes<HTMLButtonElement>['onClick']
  disabled?: boolean
  type?: ButtonHTMLAttributes<HTMLButtonElement>['type']
}

export type ChipProps = BaseChipProps & Partial<InteractiveProps>

const toneStyles: Record<ChipTone, string> = {
  primary: 'border-brand-500/60 bg-brand-500/10 text-brand-100',
  idle: 'border-slate-700 bg-slate-900/60 text-slate-200',
  info: 'border-sky-500/50 bg-sky-500/10 text-sky-100',
  warning: 'border-amber-500/60 bg-amber-500/10 text-amber-100',
  danger: 'border-rose-500/60 bg-rose-500/10 text-rose-100',
}

const variantStyles: Record<ChipVariant, string> = {
  status: 'border backdrop-blur',
  action: 'border backdrop-blur hover:brightness-110 focus-visible:outline focus-visible:outline-2 focus-visible:outline-brand-400',
}

export function Chip({
  label,
  variant = 'status',
  tone = 'idle',
  className,
  onClick,
  disabled,
  type = 'button',
}: ChipProps) {
  const baseClass = [
    'inline-flex h-8 min-h-8 items-center justify-center whitespace-nowrap rounded-full px-3 text-xs font-semibold leading-none transition-colors duration-150',
    variantStyles[variant],
    toneStyles[tone],
    onClick ? 'cursor-pointer disabled:cursor-not-allowed disabled:opacity-60' : '',
    className ?? '',
  ]
    .filter(Boolean)
    .join(' ')

  if (onClick) {
    return (
      <button type={type} className={baseClass} onClick={onClick} disabled={disabled}>
        {label}
      </button>
    )
  }

  return <span className={baseClass}>{label}</span>
}

export default Chip
