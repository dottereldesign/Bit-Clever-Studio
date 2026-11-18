export default function BackupCoverage() {
  return <details className="storage-details backup-coverage">
    <summary>What this backup includes</summary>
    <p><strong>Full library export includes:</strong> available original media, saved links and thumbnails, projects and folder memberships, tags, ratings, project notes, library clients/leads and their activity, Home tiles and saved appearance preferences.</p>
    <p><strong>Not included:</strong> My Business data, account/calendar credentials, unfinished drafts, download queues, removed items and duplicate-recovery files, browser sessions and caches. Undo/Redo history is not restored.</p>
    <p>Keep a separate backup of My Business data. Missing originals cannot be recovered from this export. Copy the entire backup folder and use Verify Backup on the copy; it checks file integrity, not a full restore.</p>
  </details>;
}
