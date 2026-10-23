import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router'
import { Markdown } from '@/components/markdown'
import { AssignmentStateBadge } from '@/components/status'
import { LabReportForm } from '@/features/assignments/LabReportForm'
import { SubmissionContent } from '@/features/assignments/SubmissionContent'
import { AuthProvider } from '@/lib/auth'
import { emptyLabReport } from '@/lib/labels'
import { LoginPage } from '@/pages/LoginPage'

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
    expect(await screen.findByText('Enter your email address')).toBeInTheDocument()
    expect(screen.getByText('Enter your password')).toBeInTheDocument()
  })

  it('shows the server error for invalid credentials', async () => {
    render(wrap(<AuthProvider><LoginPage /></AuthProvider>))
    vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify({ error: { code: 'invalid_credentials', message: 'Invalid email or password' } }), { status: 401 }))
    await userEvent.type(await screen.findByLabelText('Email'), 'a@b.co')
    await userEvent.type(screen.getByLabelText('Password'), 'wrong-pass')
    await userEvent.click(screen.getByRole('button', { name: 'Sign in' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Invalid email or password')
  })
})
