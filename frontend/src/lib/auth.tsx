import { useQuery, useQueryClient } from '@tanstack/react-query'
import * as React from 'react'
import { ApiError, setCsrfToken, UNAUTHORIZED_EVENT } from '@/api/client'
import { authApi } from '@/api/endpoints'
import type { User } from '@/types/api'
import { DEFAULT_TIMEZONE } from './utils'

interface AuthContextValue {
  user: User | null
  loading: boolean
  isAdmin: boolean
  timezone: string
  login: (email: string, password: string) => Promise<User>
  logout: () => Promise<void>
  refresh: () => Promise<unknown>
}

const AuthContext = React.createContext<AuthContextValue | null>(null)

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const qc = useQueryClient()
  const me = useQuery({
    queryKey: ['me'],
    queryFn: async () => {
      try {
        const r = await authApi.me()
        setCsrfToken(r.csrf_token)
        return r.user
      } catch (e) {
        if (e instanceof ApiError && e.status === 401) {
          setCsrfToken(null)
          return null
        }
        throw e
      }
    },
    staleTime: 5 * 60_000,
    retry: false,
  })

  React.useEffect(() => {
    const onUnauthorized = () => {
      setCsrfToken(null)
      qc.setQueryData(['me'], null)
    }
    window.addEventListener(UNAUTHORIZED_EVENT, onUnauthorized)
    return () => window.removeEventListener(UNAUTHORIZED_EVENT, onUnauthorized)
  }, [qc])

  const value = React.useMemo<AuthContextValue>(() => {
    const user = me.data ?? null
    return {
      user,
      loading: me.isLoading,
      isAdmin: user?.role === 'admin',
      timezone: user?.timezone || DEFAULT_TIMEZONE,
      login: async (email, password) => {
        const r = await authApi.login(email, password)
        setCsrfToken(r.csrf_token)
        qc.clear()
        qc.setQueryData(['me'], r.user)
        return r.user
      },
      logout: async () => {
        try {
          await authApi.logout()
        } finally {
          setCsrfToken(null)
          qc.clear()
          qc.setQueryData(['me'], null)
        }
      },
      refresh: () => me.refetch(),
    }
  }, [me, qc])

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuth() {
  const ctx = React.useContext(AuthContext)
  if (!ctx) throw new Error('useAuth must be used within AuthProvider')
  return ctx
}

/** Like useAuth, but for pages rendered only when signed in. */
export function useUser() {
  const { user, ...rest } = useAuth()
  if (!user) throw new Error('useUser requires an authenticated user')
  return { user, ...rest }
}
