// Typed API functions grouped by resource.
import { api } from './client'
import type * as T from '@/types/api'

export const authApi = {
  me: () => api.get<T.MeResponse>('/auth/me'),
  login: (login: string, password: string) => api.post<T.MeResponse>('/auth/login', { login, password }),
  logout: () => api.post<void>('/auth/logout'),
  changePassword: (current_password: string, new_password: string) =>
    api.post<void>('/auth/change-password', { current_password, new_password }),
  resetPassword: (token: string, new_password: string) => api.post<void>('/auth/reset-password', { token, new_password }),
  sessions: () => api.get<T.SessionInfo[]>('/auth/sessions'),
  revokeSession: (id: string) => api.delete(`/auth/sessions/${id}`),
  revokeOthers: () => api.post<void>('/auth/sessions/revoke-others'),
  updateProfile: (body: { name?: string; username?: string; bio?: string; timezone?: string }) => api.patch<T.User>('/me', body),
}

export const usersApi = {
  list: (q?: { role?: string; q?: string; course_id?: string }) => api.get<T.StudentListItem[]>('/users', q),
  create: (body: { email: string; username?: string; name: string; password?: string; course_ids: string[] }) =>
    api.post<T.CreateUserResponse>('/users', body),
  update: (id: string, body: Partial<Pick<T.User, 'name' | 'email' | 'is_active' | 'timezone'>> & { username?: string }) =>
    api.patch<T.User>(`/users/${id}`, body),
  resetLink: (id: string) => api.post<{ link: string; expires_at: string }>(`/users/${id}/reset-link`),
  revokeSessions: (id: string) => api.post<{ count: number }>(`/users/${id}/revoke-sessions`),
}

export type CourseInput = Partial<
  Pick<T.Course, 'title' | 'slug' | 'description' | 'difficulty' | 'duration_weeks' | 'days_per_week' | 'status' | 'objectives'>
> & { start_date?: string; end_date?: string; generate_weeks?: boolean }

export const coursesApi = {
  list: () => api.get<T.CourseListItem[]>('/courses'),
  get: (id: string) => api.get<T.CourseListItem>(`/courses/${id}`),
  create: (body: CourseInput) => api.post<{ id: string }>('/courses', body),
  update: (id: string, body: CourseInput) => api.patch<T.Course>(`/courses/${id}`, body),
  remove: (id: string) => api.delete(`/courses/${id}`),
  uploadCover: (id: string, file: File) => api.upload<T.Course>(`/courses/${id}/cover`, file),
  enrollments: (id: string) => api.get<T.EnrollmentItem[]>(`/courses/${id}/enrollments`),
  enroll: (id: string, user_ids: string[]) => api.post<void>(`/courses/${id}/enrollments`, { user_ids }),
  unenroll: (id: string, userId: string) => api.delete(`/courses/${id}/enrollments/${userId}`),
  curriculum: (id: string) => api.get<T.CurriculumResponse>(`/courses/${id}/weeks`),
  lessons: (id: string) => api.get<T.LessonRef[]>(`/courses/${id}/lessons`),
  generateWeeks: (id: string, create_day_lessons: boolean) =>
    api.post<{ count: number }>(`/courses/${id}/weeks/generate`, { create_day_lessons }),
  progress: (id: string) => api.get<T.StudentProgressRow[]>(`/courses/${id}/progress`),
  studentProgress: (id: string, studentId: string) => api.get<T.CourseProgress>(`/courses/${id}/students/${studentId}/progress`),
  gradebook: (id: string, q?: { week_id?: string }) => api.get<T.Gradebook>(`/courses/${id}/gradebook`, q),
}

export type WeekInput = Partial<Pick<T.Week, 'title' | 'description' | 'objectives' | 'instructor_notes' | 'estimated_minutes' | 'status'>> & {
  module_id?: string
  available_from?: string
  cascade_lessons?: boolean
}

export type LessonInput = Partial<
  Pick<T.Lesson, 'title' | 'day_number' | 'summary' | 'content_md' | 'lab_md' | 'objectives' | 'estimated_minutes' | 'status'>
> & { video_url?: string; prerequisite_ids?: string[] }

export const curriculumApi = {
  createModule: (courseId: string, body: { title: string; description?: string }) =>
    api.post<T.Module>(`/courses/${courseId}/modules`, body),
  updateModule: (id: string, body: { title?: string; description?: string; position?: number }) => api.patch<T.Module>(`/modules/${id}`, body),
  deleteModule: (id: string) => api.delete(`/modules/${id}`),
  createWeek: (courseId: string, body: WeekInput & { week_number?: number; create_day_lessons?: boolean }) =>
    api.post<{ id: string }>(`/courses/${courseId}/weeks`, body),
  week: (id: string) => api.get<T.WeekDetail>(`/weeks/${id}`),
  updateWeek: (id: string, body: WeekInput) => api.patch<T.Week>(`/weeks/${id}`, body),
  deleteWeek: (id: string) => api.delete(`/weeks/${id}`),
  duplicateWeek: (id: string, target_week_number: number) => api.post<{ id: string }>(`/weeks/${id}/duplicate`, { target_week_number }),
  createLesson: (weekId: string, body: LessonInput & { title: string }) => api.post<T.Lesson>(`/weeks/${weekId}/lessons`, body),
  reorderLessons: (weekId: string, lesson_ids: string[]) => api.post<void>(`/weeks/${weekId}/lessons/reorder`, { lesson_ids }),
  lesson: (id: string) => api.get<T.LessonDetail>(`/lessons/${id}`),
  updateLesson: (id: string, body: LessonInput) => api.patch<T.Lesson>(`/lessons/${id}`, body),
  deleteLesson: (id: string) => api.delete(`/lessons/${id}`),
  duplicateLesson: (id: string, target_week_id?: string) => api.post<{ id: string }>(`/lessons/${id}/duplicate`, { target_week_id }),
  complete: (id: string) => api.post<void>(`/lessons/${id}/complete`),
  uncomplete: (id: string) => api.delete(`/lessons/${id}/complete`),
}

export type AssignmentInput = Partial<Omit<T.Assignment, 'id' | 'created_at' | 'updated_at' | 'week_id' | 'lesson_id' | 'due_at' | 'publish_at'>> & {
  week_id?: string
  lesson_id?: string
  due_at?: string
  publish_at?: string
  rubric?: Omit<T.RubricCriterion, 'position'>[]
}

export const assignmentsApi = {
  list: (q?: { course_id?: string; week_id?: string; status?: string }) => api.get<T.AssignmentListItem[]>('/assignments', q),
  get: (id: string) => api.get<T.AssignmentDetail>(`/assignments/${id}`),
  create: (body: AssignmentInput) => api.post<{ id: string }>('/assignments', body),
  update: (id: string, body: AssignmentInput) => api.patch<T.Assignment>(`/assignments/${id}`, body),
  remove: (id: string) => api.delete(`/assignments/${id}`),
  mySubmission: (id: string) => api.get<T.SubmissionDetail>(`/assignments/${id}/submission`),
  saveDraft: (id: string, body: { body_md?: string; repository_url?: string; lab_report?: T.LabReport }) =>
    api.put<T.Submission>(`/assignments/${id}/submission`, body),
  uploadAttachment: (id: string, file: File) => api.upload<T.FileMeta>(`/assignments/${id}/submission/attachments`, file),
  deleteAttachment: (id: string, fileId: string) => api.delete(`/assignments/${id}/submission/attachments/${fileId}`),
  submit: (id: string) => api.post<T.Submission>(`/assignments/${id}/submissions`),
  submissions: (id: string) => api.get<T.SubmissionListItem[]>(`/assignments/${id}/submissions`),
}

export const submissionsApi = {
  list: (q?: { course_id?: string; week_id?: string; assignment_id?: string; student_id?: string; status?: string }) =>
    api.get<T.SubmissionListItem[]>('/submissions', q),
  get: (id: string) => api.get<T.SubmissionDetail>(`/submissions/${id}`),
  startReview: (id: string) => api.post<void>(`/submissions/${id}/start-review`),
  review: (id: string, body: { action: 'save' | 'return' | 'request_revision'; score?: number | null; rubric_scores?: Record<string, number>; feedback?: string }) =>
    api.post<T.SubmissionDetail>(`/submissions/${id}/review`, body),
}

export type ResourceInput = Partial<Pick<T.Resource, 'title' | 'description' | 'kind' | 'content_md' | 'visibility'>> & {
  course_id?: string
  module_id?: string
  week_id?: string
  lesson_id?: string
  file_id?: string
  url?: string
}

export const resourcesApi = {
  list: (q?: { course_id?: string; module_id?: string; week_id?: string; lesson_id?: string; kind?: string; q?: string; sort?: string }) =>
    api.get<T.Resource[]>('/resources', q),
  create: (body: ResourceInput) => api.post<T.Resource>('/resources', body),
  update: (id: string, body: ResourceInput) => api.patch<T.Resource>(`/resources/${id}`, body),
  remove: (id: string) => api.delete(`/resources/${id}`),
  uploadFile: (file: File) => api.upload<T.FileMeta>('/files', file),
}

export const announcementsApi = {
  list: (q?: { course_id?: string; limit?: number }) => api.get<T.Announcement[]>('/announcements', q),
  create: (body: { course_id: string; title: string; body_md: string; status: string; pinned: boolean }) =>
    api.post<T.Announcement>('/announcements', body),
  update: (id: string, body: Partial<{ title: string; body_md: string; status: string; pinned: boolean }>) =>
    api.patch<T.Announcement>(`/announcements/${id}`, body),
  remove: (id: string) => api.delete(`/announcements/${id}`),
}

export const notificationsApi = {
  list: () => api.get<T.NotificationList>('/notifications'),
  read: (id: string) => api.post<void>(`/notifications/${id}/read`),
  readAll: () => api.post<void>('/notifications/read-all'),
}

export type EventInput = {
  course_id?: string
  title?: string
  description?: string
  kind?: string
  starts_at?: string
  ends_at?: string
  location?: string
  /** `{ freq: 'none' }` stops repeating; omitted leaves the rule unchanged. */
  recurrence?: Omit<T.Recurrence, 'freq'> & { freq: T.RecurrenceFreq | 'none' }
  timezone?: string
}

export const calendarApi = {
  entries: (q: { from: string; to: string; course_id?: string }) => api.get<T.CalendarEntry[]>('/calendar', q),
  create: (body: EventInput) => api.post<T.CalendarEvent>('/calendar/events', body),
  update: (id: string, body: EventInput) => api.patch<T.CalendarEvent>(`/calendar/events/${id}`, body),
  remove: (id: string) => api.delete(`/calendar/events/${id}`),
  /** Removes one occurrence (YYYY-MM-DD) from a repeating event. */
  skip: (id: string, date: string) => api.post<T.CalendarEvent>(`/calendar/events/${id}/skip`, { date }),
}

export const miscApi = {
  dashboard: () => api.get<T.Dashboard>('/dashboard'),
  myProgress: () => api.get<T.CourseProgress[]>('/me/progress'),
  analytics: (course_id?: string) => api.get<T.Analytics>('/analytics', { course_id }),
  search: (q: string) => api.get<T.SearchResult[]>('/search', { q }),
  settings: () => api.get<T.PlatformSettings>('/settings'),
  updateSettings: (body: Partial<Pick<T.PlatformSettings, 'platform_name' | 'default_timezone' | 'support_email'>>) =>
    api.patch<T.PlatformSettings>('/settings', body),
  auditLogs: () => api.get<T.AuditLog[]>('/audit-logs', { limit: 200 }),
}
