import * as React from 'react'

export interface Crumb {
  label: string
  to?: string
}

const Ctx = React.createContext<{ crumbs: Crumb[]; setCrumbs: (c: Crumb[]) => void }>({ crumbs: [], setCrumbs: () => {} })

export function BreadcrumbProvider({ children }: { children: React.ReactNode }) {
  const [crumbs, setCrumbs] = React.useState<Crumb[]>([])
  return <Ctx.Provider value={{ crumbs, setCrumbs }}>{children}</Ctx.Provider>
}

export const useBreadcrumbState = () => React.useContext(Ctx).crumbs

/** Declares the breadcrumb trail for the current page. */
export function useBreadcrumbs(crumbs: Crumb[]) {
  const { setCrumbs } = React.useContext(Ctx)
  const key = JSON.stringify(crumbs)
  React.useEffect(() => {
    setCrumbs(JSON.parse(key))
    document.title = crumbs.length ? `${crumbs[crumbs.length - 1].label} · OpenSyllabus` : 'OpenSyllabus'
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, setCrumbs])
}
