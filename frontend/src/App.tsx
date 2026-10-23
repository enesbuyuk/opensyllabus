import * as React from 'react'
import { BrowserRouter, Navigate, Outlet, Route, Routes, useLocation } from 'react-router'
import { Toaster } from 'sonner'
import { PageSkeleton } from '@/components/page'
import { TooltipProvider } from '@/components/ui/tooltip'
import { AppLayout } from '@/layouts/AppLayout'
import { AuthProvider, useAuth } from '@/lib/auth'
import { ThemeProvider, useTheme } from '@/lib/theme'
import { LoginPage } from '@/pages/LoginPage'
import { ResetPasswordPage } from '@/pages/ResetPasswordPage'

const lazy = (load: () => Promise<Record<string, unknown>>, name: string) =>
  React.lazy(() => load().then((m) => ({ default: m[name] as React.ComponentType })))

const DashboardPage = lazy(() => import('@/pages/DashboardPage'), 'DashboardPage')
const CoursesPage = lazy(() => import('@/pages/CoursesPage'), 'CoursesPage')
const CourseOverviewPage = lazy(() => import('@/pages/CourseOverviewPage'), 'CourseOverviewPage')
const CurriculumPage = lazy(() => import('@/pages/CurriculumPage'), 'CurriculumPage')
const WeekPage = lazy(() => import('@/pages/WeekPage'), 'WeekPage')
const LessonPage = lazy(() => import('@/pages/LessonPage'), 'LessonPage')
const AssignmentsPage = lazy(() => import('@/pages/AssignmentsPage'), 'AssignmentsPage')
const AssignmentPage = lazy(() => import('@/pages/AssignmentPage'), 'AssignmentPage')
const CalendarPage = lazy(() => import('@/pages/CalendarPage'), 'CalendarPage')
const ResourcesPage = lazy(() => import('@/pages/ResourcesPage'), 'ResourcesPage')
const AnnouncementsPage = lazy(() => import('@/pages/AnnouncementsPage'), 'AnnouncementsPage')
const ProgressPage = lazy(() => import('@/pages/ProgressPage'), 'ProgressPage')
const SettingsPage = lazy(() => import('@/pages/SettingsPage'), 'SettingsPage')
const NotFoundPage = lazy(() => import('@/pages/NotFoundPage'), 'NotFoundPage')
const AdminCoursesPage = lazy(() => import('@/pages/admin/CoursesAdminPage'), 'CoursesAdminPage')
const CourseEditorPage = lazy(() => import('@/pages/admin/CourseEditorPage'), 'CourseEditorPage')
const WeekEditorPage = lazy(() => import('@/pages/admin/WeekEditorPage'), 'WeekEditorPage')
const LessonEditorPage = lazy(() => import('@/pages/admin/LessonEditorPage'), 'LessonEditorPage')
const AssignmentEditorPage = lazy(() => import('@/pages/admin/AssignmentEditorPage'), 'AssignmentEditorPage')
const StudentsPage = lazy(() => import('@/pages/admin/StudentsPage'), 'StudentsPage')
const ReviewsPage = lazy(() => import('@/pages/admin/ReviewsPage'), 'ReviewsPage')
const GradebookPage = lazy(() => import('@/pages/admin/GradebookPage'), 'GradebookPage')
const AnalyticsPage = lazy(() => import('@/pages/admin/AnalyticsPage'), 'AnalyticsPage')

function FullScreenLoader() {
  return (
    <div className="flex h-dvh items-center justify-center" aria-busy="true">
      <div className="size-6 animate-spin rounded-full border-2 border-muted border-t-primary" />
    </div>
  )
}

function RequireAuth() {
  const { user, loading } = useAuth()
  const location = useLocation()
  if (loading) return <FullScreenLoader />
  if (!user) return <Navigate to="/login" replace state={{ from: location.pathname + location.search }} />
  return <Outlet />
}

function RequireAdmin() {
  const { isAdmin } = useAuth()
  return isAdmin ? <Outlet /> : <Navigate to="/" replace />
}

function Suspended() {
  return (
    <React.Suspense fallback={<PageSkeleton />}>
      <Outlet />
    </React.Suspense>
  )
}

function ThemedToaster() {
  const { theme } = useTheme()
  return <Toaster theme={theme} position="bottom-right" richColors closeButton toastOptions={{ className: 'font-sans' }} />
}

export function App() {
  return (
    <ThemeProvider>
      <TooltipProvider delayDuration={300}>
        <BrowserRouter>
          <AuthProvider>
            <Routes>
              <Route path="/login" element={<LoginPage />} />
              <Route path="/reset-password" element={<ResetPasswordPage />} />
              <Route element={<RequireAuth />}>
                <Route element={<AppLayout />}>
                  <Route element={<Suspended />}>
                    <Route index element={<DashboardPage />} />
                    <Route path="courses" element={<CoursesPage />} />
                    <Route path="courses/:id" element={<CourseOverviewPage />} />
                    <Route path="curriculum" element={<CurriculumPage />} />
                    <Route path="weeks/:id" element={<WeekPage />} />
                    <Route path="lessons/:id" element={<LessonPage />} />
                    <Route path="assignments" element={<AssignmentsPage />} />
                    <Route path="assignments/:id" element={<AssignmentPage />} />
                    <Route path="calendar" element={<CalendarPage />} />
                    <Route path="resources" element={<ResourcesPage />} />
                    <Route path="announcements" element={<AnnouncementsPage />} />
                    <Route path="progress" element={<ProgressPage />} />
                    <Route path="settings" element={<SettingsPage />} />
                    <Route path="admin" element={<RequireAdmin />}>
                      <Route path="courses" element={<AdminCoursesPage />} />
                      <Route path="courses/:id" element={<CourseEditorPage />} />
                      <Route path="weeks/:id" element={<WeekEditorPage />} />
                      <Route path="lessons/:id" element={<LessonEditorPage />} />
                      <Route path="assignments/new" element={<AssignmentEditorPage />} />
                      <Route path="assignments/:id/edit" element={<AssignmentEditorPage />} />
                      <Route path="students" element={<StudentsPage />} />
                      <Route path="reviews" element={<ReviewsPage />} />
                      <Route path="gradebook" element={<GradebookPage />} />
                      <Route path="analytics" element={<AnalyticsPage />} />
                    </Route>
                    <Route path="*" element={<NotFoundPage />} />
                  </Route>
                </Route>
              </Route>
            </Routes>
          </AuthProvider>
        </BrowserRouter>
        <ThemedToaster />
      </TooltipProvider>
    </ThemeProvider>
  )
}
