import type { AssignmentKind, AttendanceSession, AttendanceStatus, CalendarEntry, Recurrence, RecurrenceFreq, ResourceKind, StudentAssignmentState, SubmissionStatus } from '@/types/api'
import { formatPlainDate } from './utils'

export const ASSIGNMENT_KIND_LABELS: Record<AssignmentKind, string> = {
  written: 'Written assignment',
  file_upload: 'File upload',
  code: 'Code submission',
  lab_report: 'Laboratory report',
  research: 'Research task',
  repository: 'Repository URL',
  mixed: 'Mixed submission',
}

export const STATE_LABELS: Record<StudentAssignmentState | 'missing', string> = {
  not_started: 'Not Started',
  draft: 'Draft',
  submitted: 'Submitted',
  late: 'Late',
  under_review: 'Under Review',
  graded: 'Graded',
  revision_requested: 'Revision Requested',
  missing: 'Missing',
}

export const SUBMISSION_STATUS_LABELS: Record<SubmissionStatus, string> = {
  not_started: 'Not Started',
  draft: 'Draft',
  submitted: 'Submitted',
  under_review: 'Under Review',
  graded: 'Graded',
  revision_requested: 'Revision Requested',
}

export const RESOURCE_KIND_LABELS: Record<ResourceKind, string> = {
  pdf: 'PDF',
  image: 'Image',
  markdown: 'Notes',
  code: 'Code example',
  link: 'External link',
  video: 'Video',
  datasheet: 'Datasheet',
  lab: 'Lab instructions',
}

export const LATE_POLICY_LABELS = {
  accept: 'Accept late submissions',
  penalty: 'Accept with penalty',
  reject: 'Do not accept late submissions',
} as const

export const CALENDAR_KIND_LABELS: Record<CalendarEntry['kind'], string> = {
  session: 'Session',
  lab: 'Laboratory',
  milestone: 'Milestone',
  other: 'Event',
  deadline: 'Deadline',
  course_start: 'Course start',
  course_end: 'Course end',
  week_start: 'Week start',
  class: 'Class',
}

export const LAB_REPORT_SECTIONS: { key: keyof import('@/types/api').LabReport; label: string; hint: string; required?: boolean; rows: number }[] = [
  { key: 'experiment_title', label: 'Experiment title', hint: 'A concise, descriptive title.', required: true, rows: 1 },
  { key: 'objective', label: 'Objective', hint: 'What the experiment sets out to determine or verify.', required: true, rows: 3 },
  { key: 'equipment', label: 'Equipment and components', hint: 'Instruments, components and values (e.g. 1 kΩ resistor, DMM).', rows: 4 },
  { key: 'setup', label: 'Circuit diagram or setup', hint: 'Describe the setup; attach schematics or photos below.', rows: 4 },
  { key: 'procedure', label: 'Experimental procedure', hint: 'Numbered steps someone else could reproduce.', required: true, rows: 6 },
  { key: 'measurements', label: 'Measurements', hint: 'Tables of measured values (Markdown tables work). Attach raw CSV files.', rows: 6 },
  { key: 'observations', label: 'Observations', hint: 'What you noticed during the experiment.', rows: 4 },
  { key: 'results', label: 'Results', hint: 'Processed data, calculated values and plots.', required: true, rows: 5 },
  { key: 'discussion', label: 'Discussion', hint: 'Interpretation, sources of error and comparison with theory.', rows: 5 },
  { key: 'conclusion', label: 'Conclusion', hint: 'The main takeaway in a few sentences.', required: true, rows: 3 },
]

export function emptyLabReport() {
  return Object.fromEntries(LAB_REPORT_SECTIONS.map((s) => [s.key, ''])) as unknown as import('@/types/api').LabReport
}

export const WEEKDAYS = [
  { n: 1, short: 'Mon', long: 'Monday', letter: 'M' },
  { n: 2, short: 'Tue', long: 'Tuesday', letter: 'T' },
  { n: 3, short: 'Wed', long: 'Wednesday', letter: 'W' },
  { n: 4, short: 'Thu', long: 'Thursday', letter: 'T' },
  { n: 5, short: 'Fri', long: 'Friday', letter: 'F' },
  { n: 6, short: 'Sat', long: 'Saturday', letter: 'S' },
  { n: 7, short: 'Sun', long: 'Sunday', letter: 'S' },
]

export const FREQ_UNIT: Record<RecurrenceFreq, [string, string]> = { daily: ['day', 'days'], weekly: ['week', 'weeks'], monthly: ['month', 'months'] }

/** "Every 2 weeks on Mon, Wed, 4 times" */
export function describeRecurrence(r: Recurrence) {
  const [one, many] = FREQ_UNIT[r.freq]
  let text = r.interval === 1 ? `Every ${one}` : `Every ${r.interval} ${many}`
  if (r.freq === 'weekly' && r.weekdays.length) text += ` on ${r.weekdays.map((d) => WEEKDAYS[d - 1].short).join(', ')}`
  if (r.until) text += `, until ${formatPlainDate(r.until)}`
  if (r.count) text += `, ${r.count} ${r.count === 1 ? 'time' : 'times'}`
  return text
}

export const ATTENDANCE_LABELS: Record<AttendanceStatus, string> = {
  present: 'Present',
  late: 'Late',
  absent: 'Absent',
  excused: 'Excused',
}

export const ATTENDANCE_STYLE: Record<AttendanceStatus, string> = {
  present: 'border-success/40 bg-success/15 text-success',
  late: 'border-warning/40 bg-warning/15 text-warning',
  absent: 'border-destructive/40 bg-destructive/15 text-destructive',
  excused: 'border-info/40 bg-info/15 text-info',
}

export function attendanceSessionLabel(s: AttendanceSession) {
  const when = s.date ? formatPlainDate(s.date, { weekday: 'short' }) : `Day ${s.day_number}`
  return `Week ${s.week_number} · ${when}`
}
