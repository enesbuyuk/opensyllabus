// Types mirroring the backend JSON contracts (see /api/docs for the OpenAPI schema).

export type Role = 'admin' | 'student'

export interface User {
  id: string
  email: string
  name: string
  role: Role
  timezone: string
  bio: string
  is_active: boolean
  must_change_password: boolean
  last_login_at: string | null
  created_at: string
  updated_at: string
}

export interface UserSummary {
  id: string
  name: string
  email: string
}

export interface MeResponse {
  user: User
  csrf_token: string
}

export interface SessionInfo {
  id: string
  ip_address: string | null
  user_agent: string | null
  created_at: string
  last_seen_at: string
  expires_at: string
  current: boolean
}

export interface CourseRef {
  id: string
  title: string
  slug: string
}

export interface StudentListItem extends User {
  courses: CourseRef[]
}

export interface CreateUserResponse {
  user: User
  setup_link: string | null
}

export type CourseStatus = 'draft' | 'published' | 'archived'
export type Difficulty = 'beginner' | 'intermediate' | 'advanced'

export interface Course {
  id: string
  title: string
  slug: string
  description: string
  cover_file_id: string | null
  difficulty: Difficulty
  duration_weeks: number
  days_per_week: number
  instructor_id: string | null
  status: CourseStatus
  start_date: string | null
  end_date: string | null
  objectives: string[]
  created_at: string
  updated_at: string
}

export interface CourseListItem extends Course {
  instructor_name: string | null
  enrolled_count: number
  published_weeks: number
  total_weeks: number
  completion_percent: number | null
  current_week: number | null
}

export interface EnrollmentItem {
  user_id: string
  name: string
  email: string
  is_active: boolean
  enrolled_at: string
}

export type PublishStatus = 'draft' | 'published'

export interface Module {
  id: string
  course_id: string
  title: string
  description: string
  position: number
}

export interface Week {
  id: string
  course_id: string
  module_id: string | null
  week_number: number
  title: string
  description: string
  objectives: string[]
  instructor_notes: string
  estimated_minutes: number
  status: PublishStatus
  available_from: string | null
  created_at: string
  updated_at: string
}

export type WeekState = 'completed' | 'current' | 'available' | 'locked'

export interface WeekSummary extends Week {
  state: WeekState
  lesson_count: number
  published_lesson_count: number
  completed_lesson_count: number
  assignment_count: number
  completion_percent: number
  starts_on: string | null
}

export interface CurriculumResponse {
  course_id: string
  current_week: number | null
  modules: Module[]
  weeks: WeekSummary[]
}

export interface Lesson {
  id: string
  week_id: string
  day_number: number
  position: number
  title: string
  summary: string
  content_md: string
  lab_md: string
  video_url: string | null
  objectives: string[]
  estimated_minutes: number
  status: PublishStatus
  published_at: string | null
  created_at: string
  updated_at: string
}

export interface LessonListItem extends Lesson {
  completed: boolean
  locked_by_prerequisites: boolean
}

export interface LessonRef {
  id: string
  title: string
  week_number: number
  day_number: number
  completed: boolean
}

export interface WeekDetail extends WeekSummary {
  module: Module | null
  lessons: LessonListItem[]
  assignments: AssignmentListItem[]
  resources: Resource[]
  previous_week_id: string | null
  next_week_id: string | null
}

export interface LessonDetail extends Lesson {
  course_id: string
  week_number: number
  week_title: string
  week_status: PublishStatus
  completed: boolean
  completed_at: string | null
  prerequisites: LessonRef[]
  prerequisites_met: boolean
  resources: Resource[]
  assignments: AssignmentListItem[]
  previous_lesson_id: string | null
  next_lesson_id: string | null
}

export type AssignmentKind = 'written' | 'file_upload' | 'code' | 'lab_report' | 'research' | 'repository' | 'mixed'
export type LatePolicy = 'accept' | 'penalty' | 'reject'
export type AssignmentStatus = 'draft' | 'published' | 'archived'
export type StudentAssignmentState =
  | 'not_started'
  | 'draft'
  | 'submitted'
  | 'late'
  | 'under_review'
  | 'graded'
  | 'revision_requested'

export interface Assignment {
  id: string
  course_id: string
  week_id: string | null
  lesson_id: string | null
  title: string
  description: string
  instructions_md: string
  reference_md: string
  kind: AssignmentKind
  max_grade: number
  due_at: string | null
  publish_at: string | null
  late_policy: LatePolicy
  late_penalty_percent: number
  allowed_file_types: string[]
  max_upload_mb: number
  allow_resubmission: boolean
  status: AssignmentStatus
  created_at: string
  updated_at: string
}

export interface RubricCriterion {
  id: string
  position: number
  title: string
  description: string
  max_points: number
}

export interface AssignmentListItem extends Assignment {
  course_title: string
  week_number: number | null
  week_title: string | null
  lesson_title: string | null
  state: StudentAssignmentState | null
  score: number | null
  submitted_at: string | null
  submission_count: number | null
  pending_review_count: number | null
}

export interface AssignmentDetail extends AssignmentListItem {
  rubric: RubricCriterion[]
  is_past_due: boolean
  accepts_submissions: boolean
}

export interface LabReport {
  experiment_title: string
  objective: string
  equipment: string
  setup: string
  procedure: string
  measurements: string
  observations: string
  results: string
  discussion: string
  conclusion: string
}

export type SubmissionStatus = 'not_started' | 'draft' | 'submitted' | 'under_review' | 'graded' | 'revision_requested'

export interface FileMeta {
  id: string
  original_name: string
  content_type: string
  size_bytes: number
  created_at: string
}

export interface Submission {
  id: string
  assignment_id: string
  student_id: string
  status: SubmissionStatus
  body_md: string
  repository_url: string | null
  lab_report: LabReport | null
  attempt: number
  submitted_at: string | null
  is_late: boolean
  created_at: string
  updated_at: string
}

export interface SubmissionRevision {
  id: string
  attempt: number
  body_md: string
  repository_url: string | null
  lab_report: LabReport | null
  attachments: FileMeta[]
  submitted_at: string
  is_late: boolean
}

export interface Grade {
  id: string
  submission_id: string
  score: number
  max_score: number
  rubric_scores: Record<string, number>
  late_penalty: number
  graded_by: string | null
  graded_at: string
  released_at: string | null
}

export interface Feedback {
  id: string
  submission_id: string
  author_id: string | null
  author_name: string | null
  attempt: number
  kind: 'comment' | 'revision_request'
  body_md: string
  released_at: string | null
  created_at: string
}

export interface SubmissionDetail extends Submission {
  student: UserSummary
  assignment: Assignment
  course_title: string
  week_number: number | null
  rubric: RubricCriterion[]
  attachments: FileMeta[]
  revisions: SubmissionRevision[]
  grade: Grade | null
  feedback: Feedback[]
  can_edit: boolean
}

export interface SubmissionListItem {
  id: string
  assignment_id: string
  assignment_title: string
  course_id: string
  course_title: string
  week_id: string | null
  week_number: number | null
  student_id: string
  student_name: string
  status: SubmissionStatus
  attempt: number
  submitted_at: string | null
  is_late: boolean
  due_at: string | null
  max_grade: number
  score: number | null
  released_at: string | null
  updated_at: string
}

export type ResourceKind = 'pdf' | 'image' | 'markdown' | 'code' | 'link' | 'video' | 'datasheet' | 'lab'

export interface Resource {
  id: string
  course_id: string
  module_id: string | null
  week_id: string | null
  lesson_id: string | null
  title: string
  description: string
  kind: ResourceKind
  file_id: string | null
  url: string | null
  content_md: string
  visibility: 'students' | 'instructor'
  created_at: string
  updated_at: string
  course_title: string
  week_number: number | null
  module_title: string | null
  lesson_title: string | null
  file_name: string | null
  file_size: number | null
  file_type: string | null
}

export interface Announcement {
  id: string
  course_id: string
  course_title: string
  author_id: string | null
  author_name: string | null
  title: string
  body_md: string
  status: PublishStatus
  pinned: boolean
  published_at: string | null
  created_at: string
  updated_at: string
}

export interface Notification {
  id: string
  kind: string
  title: string
  body: string
  link: string | null
  read_at: string | null
  created_at: string
}

export interface NotificationList {
  items: Notification[]
  unread_count: number
}

export type CalendarEventKind = 'session' | 'lab' | 'milestone' | 'other'

export interface CalendarEvent {
  id: string
  course_id: string
  course_title: string
  title: string
  description: string
  kind: CalendarEventKind
  starts_at: string
  ends_at: string | null
  location: string
}

export interface CalendarEntry {
  id: string
  kind: CalendarEventKind | 'deadline' | 'course_start' | 'course_end' | 'week_start'
  title: string
  description: string
  starts_at: string
  ends_at: string | null
  all_day: boolean
  location: string
  course_id: string
  course_title: string
  link: string | null
  event_id: string | null
}

export interface WeekProgress {
  week_id: string
  week_number: number
  title: string
  state: WeekState
  lessons_total: number
  lessons_completed: number
  assignments_total: number
  assignments_completed: number
  completion_percent: number
}

export interface GradeEntry {
  assignment_id: string
  assignment_title: string
  week_number: number | null
  score: number
  max_score: number
  percent: number
  released_at: string
}

export interface CourseProgress {
  course_id: string
  course_title: string
  student_id: string
  current_week: number | null
  lessons_total: number
  lessons_completed: number
  assignments_total: number
  assignments_completed: number
  assignments_awaiting_review: number
  assignments_pending: number
  assignments_late: number
  completion_percent: number
  lesson_percent: number
  assignment_percent: number
  average_grade_percent: number | null
  grades: GradeEntry[]
  weeks: WeekProgress[]
}

export interface StudentProgressRow {
  student: UserSummary
  last_login_at: string | null
  progress: CourseProgress
}

export interface GradebookColumn {
  id: string
  title: string
  week_number: number | null
  max_grade: number
  due_at: string | null
  status: AssignmentStatus
}

export interface GradebookCell {
  submission_id: string | null
  state: StudentAssignmentState | 'missing'
  score: number | null
  released: boolean
  is_late: boolean
  attempt: number
}

export interface GradebookRow {
  student: UserSummary
  completion_percent: number
  lesson_percent: number
  average_grade_percent: number | null
  cells: Record<string, GradebookCell>
  weeks: Record<string, number>
}

export interface Gradebook {
  course_id: string
  course_title: string
  columns: GradebookColumn[]
  week_numbers: number[]
  rows: GradebookRow[]
}

export interface DashboardLesson {
  id: string
  title: string
  summary: string
  day_number: number
  estimated_minutes: number
  completed: boolean
}

export interface StudentDashboard {
  role: 'student'
  courses: CourseProgress[]
  current_week: {
    course_id: string
    course_title: string
    week_id: string
    week_number: number
    title: string
    description: string
    completion_percent: number
    lessons: DashboardLesson[]
  } | null
  continue_learning: { lesson_id: string; lesson_title: string; week_number: number; day_number: number } | null
  pending_assignments: AssignmentListItem[]
  upcoming_deadlines: AssignmentListItem[]
  announcements: Announcement[]
  latest_grades: GradeEntry[]
}

export interface ActivityItem {
  kind: 'submission' | 'lesson_completed'
  student_name: string
  title: string
  link: string
  at: string
}

export interface AdminDashboard {
  role: 'admin'
  total_students: number
  active_courses: number
  pending_submissions: number
  overdue_assignments: number
  average_completion: number | null
  recent_activity: ActivityItem[]
  upcoming_deadlines: AssignmentListItem[]
  review_queue: SubmissionListItem[]
  student_overview: {
    student_id: string
    name: string
    course_id: string
    course_title: string
    completion_percent: number
    average_grade_percent: number | null
    assignments_pending: number
    last_login_at: string | null
  }[]
  weekly_activity: { week_start: string; submissions: number; lessons_completed: number }[]
}

export type Dashboard = StudentDashboard | AdminDashboard

export interface Analytics {
  course_id: string | null
  course_title: string | null
  students: {
    student_id: string
    name: string
    completion_percent: number
    lesson_percent: number
    assignment_percent: number
    average_grade_percent: number | null
    late_count: number
  }[]
  assignments: {
    assignment_id: string
    title: string
    week_number: number | null
    enrolled: number
    submitted: number
    graded: number
    late: number
    average_percent: number | null
  }[]
  submission_status: { status: string; count: number }[]
  weeks: { week_number: number; title: string; average_completion: number }[]
}

export interface SearchResult {
  kind: 'course' | 'week' | 'lesson' | 'assignment' | 'resource' | 'announcement' | 'student'
  id: string
  title: string
  subtitle: string
  link: string
}

export interface PlatformSettings {
  platform_name: string
  default_timezone: string
  support_email: string
  supported_file_types: string[]
  max_upload_mb: number
}

export interface AuditLog {
  id: string
  actor_id: string | null
  actor_name: string | null
  action: string
  entity_type: string
  entity_id: string | null
  details: string
  ip_address: string | null
  created_at: string
}

export interface ApiErrorBody {
  error: { code: string; message: string; fields?: Record<string, string> }
}
