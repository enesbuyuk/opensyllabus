import { AlertDialog as P } from 'radix-ui'
import * as React from 'react'
import { buttonVariants } from './button'
import { cn } from '@/lib/utils'

interface ConfirmProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  description?: React.ReactNode
  confirmLabel?: string
  destructive?: boolean
  loading?: boolean
  onConfirm: () => void
}

/** Accessible confirmation dialog for destructive or irreversible actions. */
export function ConfirmDialog({ open, onOpenChange, title, description, confirmLabel = 'Confirm', destructive, loading, onConfirm }: ConfirmProps) {
  return (
    <P.Root open={open} onOpenChange={onOpenChange}>
      <P.Portal>
        <P.Overlay className="fixed inset-0 z-50 bg-black/60 backdrop-blur-[2px] data-[state=open]:animate-fade-in" />
        <P.Content className="fixed top-1/2 left-1/2 z-50 grid w-[calc(100%-2rem)] max-w-md -translate-x-1/2 -translate-y-1/2 gap-4 rounded-xl border bg-popover p-6 shadow-2xl data-[state=open]:animate-slide-up">
          <div className="space-y-1.5">
            <P.Title className="text-base font-semibold">{title}</P.Title>
            {description && <P.Description className="text-sm text-muted-foreground">{description}</P.Description>}
          </div>
          <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
            <P.Cancel className={buttonVariants({ variant: 'outline' })}>Cancel</P.Cancel>
            <P.Action
              className={cn(buttonVariants({ variant: destructive ? 'destructive' : 'default' }))}
              disabled={loading}
              onClick={(e) => {
                e.preventDefault()
                onConfirm()
              }}
            >
              {confirmLabel}
            </P.Action>
          </div>
        </P.Content>
      </P.Portal>
    </P.Root>
  )
}
