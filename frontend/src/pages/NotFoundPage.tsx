import { Compass } from 'lucide-react'
import { Link } from 'react-router'
import { EmptyState } from '@/components/page'
import { Button } from '@/components/ui/button'
import { useBreadcrumbs } from '@/lib/breadcrumbs'

export function NotFoundPage() {
  useBreadcrumbs([{ label: 'Not found' }])
  return (
    <EmptyState
      icon={Compass}
      title="Page not found"
      description="The page you are looking for does not exist or you do not have access to it."
      action={
        <Button asChild variant="outline">
          <Link to="/">Back to dashboard</Link>
        </Button>
      }
      className="mt-16"
    />
  )
}
