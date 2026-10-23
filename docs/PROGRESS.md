# Progress, completion and grading rules

These rules are implemented in `backend/src/services/progress.rs` and used consistently by the
dashboard, curriculum timeline, progress page, gradebook and analytics.

## What counts

| Item | Counts toward a student's progress when… |
|------|------------------------------------------|
| Lesson | the lesson is **published**, its week is **published**, and the week's optional *available from* date has been reached (platform timezone). |
| Assignment | the assignment is **published** and its *publication date* (if any) has passed. Archived and draft assignments never count. |

## What "complete" means

- A **lesson** is complete when the student marks it complete (only possible once its published
  prerequisites are complete). Students can undo this.
- An **assignment** is complete only when the instructor has **returned the review with a grade**
  (status *Graded*). Saving a draft, uploading a file or submitting is **not** completion; a revision
  request re-opens the assignment and it stops counting until it is graded again.

## Percentages

```
course completion % = (completed lessons + completed assignments) / (countable lessons + countable assignments)
lesson %            = completed lessons / countable lessons
assignment %        = completed assignments / countable assignments
week completion %   = same formula restricted to the week's lessons and assignments
```

All percentages are rounded to one decimal; an empty denominator yields 0 %. Publishing new content
increases the denominator, so a student's percentage can go down when new work becomes available.
This is intentional: it reflects how much of the *available* work is done.

## Grades (separate from completion)

- Grades become visible to the student only when the review is **returned**. Draft reviews,
  unreleased scores and unreleased feedback are never sent to students.
- **Average grade %** is the mean of `score / max score` over returned grades.
- **Late penalty:** if the submission was late and the assignment's policy is *Accept with penalty*,
  the stored score is `raw × (1 − penalty%)`; the deducted amount is stored and shown.
- With a rubric, the score is the sum of the criterion scores, each bounded by its maximum. Rubric
  maxima must add up to the assignment's maximum grade.

## Week states on the timeline

| State | Rule |
|-------|------|
| Locked | week is a draft, or its *available from* date is in the future |
| Completed | open, has at least one countable item, and every countable item is complete |
| Current | open and equal to the current week |
| Available | any other open week |

**Current week:** if the course has a start date, `floor((today − start) / 7) + 1` capped to the
course duration (none before the start date). Without a start date, the first open week that is
not completed.

## Assignment states shown to students

| State | Meaning |
|-------|---------|
| Not Started | no saved work |
| Draft | work saved but not submitted |
| Submitted | submitted on time, waiting for review |
| Late | submitted after the due date, waiting for review |
| Under Review | the instructor opened or drafted a review |
| Graded | review returned with a grade |
| Revision Requested | feedback returned; the student may edit and resubmit (even after the deadline) |

The gradebook additionally shows **Missing** for published assignments past their due date with no
submission.
