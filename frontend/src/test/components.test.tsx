import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Navigate, Route, Routes } from 'react-router'
import { Markdown } from '@/components/markdown'
import { AssignmentStateBadge } from '@/components/status'
import { LabReportForm } from '@/features/assignments/LabReportForm'
import { SubmissionContent } from '@/features/assignments/SubmissionContent'
import { AuthProvider, useAuth } from '@/lib/auth'
import { emptyLabReport } from '@/lib/labels'
import { ScheduleEditor } from '@/features/courses/ScheduleEditor'
import { CalendarPage } from '@/pages/CalendarPage'
import { LoginPage } from '@/pages/LoginPage'
import type { CourseListItem } from '@/types/api'

function wrap(ui: React.ReactNode) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return (
    <QueryClientProvider client={qc}>
      <MemoryRouter>{ui}</MemoryRouter>
    </QueryClientProvider>
  )
}

describe('Markdown', () => {
  it('renders Markdown but never raw HTML or scripts', () => {
    const { container } = render(<Markdown>{'# Title\n\n<script>alert(1)</script><img src=x onerror=alert(1)>\n\n[link](javascript:alert(1))'}</Markdown>)
    expect(screen.getByRole('heading', { name: 'Title' })).toBeInTheDocument()
    expect(container.querySelector('script')).toBeNull()
    expect(container.querySelector('img[onerror]')).toBeNull()
    expect(container.querySelector('a')?.getAttribute('href') ?? '').not.toMatch(/javascript:/)
  })

  it('opens external links safely', () => {
    render(<Markdown>{'[docs](https://example.com)'}</Markdown>)
    const a = screen.getByRole('link', { name: 'docs' })
    expect(a).toHaveAttribute('target', '_blank')
    expect(a.getAttribute('rel')).toContain('noopener')
  })
})

describe('AssignmentStateBadge', () => {
  it.each([
    ['not_started', 'Not Started'],
    ['revision_requested', 'Revision Requested'],
    ['under_review', 'Under Review'],
    ['graded', 'Graded'],
  ] as const)('shows %s as “%s”', (state, label) => {
    render(<AssignmentStateBadge state={state} />)
    expect(screen.getByText(label)).toBeInTheDocument()
  })
})

describe('Laboratory report', () => {
  it('renders every section of the template and reports edits', async () => {
    const onChange = vi.fn()
    render(<LabReportForm value={emptyLabReport()} onChange={onChange} />)
    for (const label of ['Experiment title', 'Objective', 'Equipment and components', 'Measurements', 'Discussion', 'Conclusion']) {
      expect(screen.getAllByText(label).length).toBeGreaterThan(0)
    }
    await userEvent.type(screen.getByLabelText(/Experiment title/), 'O')
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ experiment_title: 'O' }))
  })

  it('shows only filled sections in read-only view', () => {
    render(<SubmissionContent body="" labReport={{ ...emptyLabReport(), objective: 'Verify Ohm law' }} repositoryUrl={null} attachments={[]} />)
    expect(screen.getByText('Verify Ohm law')).toBeInTheDocument()
    expect(screen.queryByText('Conclusion')).toBeNull()
  })
})

describe('LoginPage', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ error: { code: 'unauthorized', message: 'authentication required' } }), { status: 401 })))
  })
  afterEach(() => vi.unstubAllGlobals())

  it('validates input before submitting', async () => {
    render(wrap(<AuthProvider><LoginPage /></AuthProvider>))
    await userEvent.click(await screen.findByRole('button', { name: 'Sign in' }))
    expect(await screen.findByText('Enter your email or username')).toBeInTheDocument()
    expect(screen.getByText('Enter your password')).toBeInTheDocument()
  })

  it('shows the server error for invalid credentials', async () => {
    render(wrap(<AuthProvider><LoginPage /></AuthProvider>))
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify({ error: { code: 'invalid_credentials', message: 'Invalid email, username or password' } }), { status: 401 }))
    await userEvent.type(await screen.findByLabelText('Email or username'), 'a@b.co')
    await userEvent.type(screen.getByLabelText('Password'), 'wrong-pass')
    await userEvent.click(screen.getByRole('button', { name: 'Sign in' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Invalid email, username or password')
  })

  it('lands on the protected page right after signing in', async () => {
    function Protected() {
      const { user, loading } = useAuth()
      if (loading) return null
      return user ? <p>Welcome {user.name}</p> : <Navigate to="/login" replace />
    }
    const user = { id: 'u1', email: 'a@b.co', username: 'ayse', name: 'Ayşe', role: 'student' }
    // Signed out until the login call; public endpoints (platform settings) answer normally.
    vi.mocked(fetch).mockImplementation(async (input) => {
      const url = String(input)
      if (url.endsWith('/auth/login')) return new Response(JSON.stringify({ user, csrf_token: 't' }), { status: 200 })
      if (url.endsWith('/auth/me')) return new Response(JSON.stringify({ error: { code: 'unauthorized', message: 'authentication required' } }), { status: 401 })
      return new Response('{}', { status: 200 })
    })
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={qc}>
        <MemoryRouter initialEntries={['/']}>
          <AuthProvider>
            <Routes>
              <Route path="/login" element={<LoginPage />} />
              <Route path="/" element={<Protected />} />
            </Routes>
          </AuthProvider>
        </MemoryRouter>
      </QueryClientProvider>,
    )
    await userEvent.type(await screen.findByLabelText('Email or username'), 'ayse')
    await userEvent.type(screen.getByLabelText('Password'), 'student-pass-1')
    await userEvent.click(screen.getByRole('button', { name: 'Sign in' }))
    // Signing in navigates twice (navigate() and LoginPage's <Navigate>), which can remount the page: query the live DOM.
    await waitFor(() => expect(screen.getByText('Welcome Ayşe')).toBeInTheDocument())
    const [, init] = vi.mocked(fetch).mock.calls.find(([url]) => String(url).endsWith('/auth/login'))!
    expect(JSON.parse(String(init?.body))).toEqual({ login: 'ayse', password: 'student-pass-1' })
  })
})

describe('CalendarPage', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('renders for an instructor and opens the repeat options of a new event', async () => {
    const admin = { id: 'a1', email: 'i@b.co', username: 'hoca', name: 'Hoca', role: 'admin', timezone: 'Europe/Istanbul' }
    const course = { id: 'c1', title: 'Robot Eli', status: 'published' }
    const series = {
      id: 'event:e1:2026-10-21', kind: 'lab', title: 'Lab', description: '', starts_at: '2026-10-21T07:00:00.000Z', ends_at: null, all_day: false,
      location: '', course_id: 'c1', course_title: 'Robot Eli', link: null, event_id: 'e1',
      series: { recurrence: { freq: 'weekly', interval: 1, weekdays: [1, 3], until: null, count: null }, starts_at: '2026-10-19T07:00:00.000Z', ends_at: null, timezone: 'Europe/Istanbul', occurrence_date: '2026-10-21' },
    }
    vi.stubGlobal('fetch', vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input)
      const cls = { ...series, id: 'class:l1', kind: 'class', title: 'W1 · Day 1', event_id: null, series: null, link: '/lessons/l1', starts_at: '2026-10-19T07:00:00.000Z' }
      const body = url.includes('/auth/me') ? { user: admin, csrf_token: 't' } : url.includes('/courses') ? [course] : url.includes('/calendar') ? [cls, series] : {}
      return new Response(JSON.stringify(body), { status: 200 })
    }))
    render(wrap(<AuthProvider><CalendarPage /></AuthProvider>))
    await userEvent.click(await screen.findByRole('tab', { name: 'Agenda' }))
    expect(await screen.findByText(/Every week on Mon, Wed/)).toBeInTheDocument()
    expect(screen.getByRole('link', { name: 'W1 · Day 1' })).toHaveAttribute('href', '/lessons/l1')

    await userEvent.click(screen.getByRole('button', { name: /New event/ }))
    await userEvent.selectOptions(screen.getByLabelText('Repeat'), 'weekly')
    expect(screen.getByRole('group', { name: 'On' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Mon' })).toHaveAttribute('aria-pressed', 'true')
    await userEvent.selectOptions(screen.getByLabelText('Repeat ends'), 'count')
    expect(screen.getByLabelText('Occurrences')).toHaveValue(10)
  })
})

describe('ScheduleEditor', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('loads the schedule, validates times and saves the enabled days', async () => {
    const admin = { id: 'a1', email: 'i@b.co', username: null, name: 'Hoca', role: 'admin', timezone: 'Europe/Istanbul' }
    const saved = { timezone: 'Europe/Istanbul', slots: [{ day_number: 1, weekday: 1, start_time: '10:00', end_time: '12:00', location: 'Lab 2' }] }
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input)
      if (url.includes('/auth/me')) return new Response(JSON.stringify({ user: admin, csrf_token: 't' }), { status: 200 })
      if (url.includes('/schedule') && init?.method === 'PUT') return new Response(String(init.body), { status: 200 })
      if (url.includes('/schedule')) return new Response(JSON.stringify(saved), { status: 200 })
      return new Response('{}', { status: 200 })
    })
    vi.stubGlobal('fetch', fetchMock)
    const course = { id: 'c1', title: 'Robot Eli', days_per_week: 3, start_date: '2026-10-19' } as unknown as CourseListItem
    render(wrap(<AuthProvider><ScheduleEditor course={course} /></AuthProvider>))

    const day1 = await screen.findByRole('group', { name: 'Day 1' })
    expect(within(day1).getByLabelText('Location')).toHaveValue('Lab 2')
    const day2 = screen.getByRole('group', { name: 'Day 2' })
    expect(within(day2).getByLabelText('Starts')).toBeDisabled()

    await userEvent.click(screen.getByRole('checkbox', { name: 'Schedule day 2' }))
    const end = within(day2).getByLabelText('Ends')
    await userEvent.clear(end)
    await userEvent.type(end, '09:00')
    expect(within(day2).getByRole('alert')).toHaveTextContent('End must be after the start')
    expect(screen.getByRole('button', { name: 'Save schedule' })).toBeDisabled()

    await userEvent.clear(end)
    await userEvent.type(end, '16:00')
    await userEvent.selectOptions(within(day2).getByLabelText('Weekday'), 'Wednesday')
    await userEvent.click(screen.getByRole('button', { name: 'Save schedule' }))
    await waitFor(() => expect(fetchMock.mock.calls.some(([, i]) => i?.method === 'PUT')).toBe(true))
    const [, init] = fetchMock.mock.calls.find(([, i]) => i?.method === 'PUT')!
    expect(JSON.parse(String(init?.body))).toEqual({
      timezone: 'Europe/Istanbul',
      slots: [
        { day_number: 1, weekday: 1, start_time: '10:00', end_time: '12:00', location: 'Lab 2' },
        { day_number: 2, weekday: 3, start_time: '10:00', end_time: '16:00', location: '' },
      ],
    })
  })
})
