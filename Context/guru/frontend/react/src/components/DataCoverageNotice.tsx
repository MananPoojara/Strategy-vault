import type {OwnerIssue as SummaryIssue} from '../services/summary-loader';
export function DataCoverageNotice({issues}: {issues?: SummaryIssue[]}) {
  if (!issues?.length) return null;
  const notes = [...new Set(issues.map(issue => issue.note))];
  const dateProblem = issues.some(issue => ['empty_range', 'invalid_range'].includes(issue.kind));
  return <section className="summary-unavailable" data-summary-unavailable role="status">
    <strong>Data coverage</strong>
    {notes.map(note => <p key={note}>{note}</p>)}
    {dateProblem ? <button className="dbtn sm" type="button" data-summary-reset-dates
      onClick={()=>window.VaultRuntime.patch({from:'',to:'',tab:'sharpe',library_page:'1'})}>Use available history</button> : null}
  </section>;
}
