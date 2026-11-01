import {
  BarChart3,
  BookOpen,
  CalendarCheck,
  CalendarDays,
  ClipboardCheck,
  FolderOpen,
  GraduationCap,
  LayoutDashboard,
  ListChecks,
  Megaphone,
  Settings,
  Sheet,
  TrendingUp,
  Users,
  type LucideIcon,
} from 'lucide-react'

export interface NavItem {
  label: string
  to: string
  icon: LucideIcon
  end?: boolean
}

export const MAIN_NAV: NavItem[] = [
  { label: 'Dashboard', to: '/', icon: LayoutDashboard, end: true },
  { label: 'My Courses', to: '/courses', icon: GraduationCap },
  { label: 'Curriculum', to: '/curriculum', icon: BookOpen },
  { label: 'Assignments', to: '/assignments', icon: ListChecks },
  { label: 'Calendar', to: '/calendar', icon: CalendarDays },
  { label: 'Resources', to: '/resources', icon: FolderOpen },
  { label: 'Announcements', to: '/announcements', icon: Megaphone },
  { label: 'Progress', to: '/progress', icon: TrendingUp },
  { label: 'Settings', to: '/settings', icon: Settings },
]

export const ADMIN_NAV: NavItem[] = [
  { label: 'Course Management', to: '/admin/courses', icon: GraduationCap },
  { label: 'Student Management', to: '/admin/students', icon: Users },
  { label: 'Submission Reviews', to: '/admin/reviews', icon: ClipboardCheck },
  { label: 'Gradebook', to: '/admin/gradebook', icon: Sheet },
  { label: 'Attendance', to: '/admin/attendance', icon: CalendarCheck },
  { label: 'Analytics', to: '/admin/analytics', icon: BarChart3 },
]
