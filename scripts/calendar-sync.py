#!/usr/bin/env python3
"""Read the desktop calendar through the existing Workspace MCP credentials.

No credential copies, write APIs, or new OAuth grants. Google expands recurring
events and exceptions. The app owns cache writes and account-change fencing.
"""
from datetime import datetime, date, timedelta, timezone
from pathlib import Path
import argparse
import json
import os
import sys
import re
import html
from urllib.parse import urlparse

ROOT = Path.home() / '.local/share/workspace-mcp'
sys.path.insert(0, str(ROOT))

def calendar_window(month):
    first = date.fromisoformat(month + '-01')
    start = first - timedelta(days=first.weekday())
    # Include the Monday after the final week: Sunday also shows the next day's agenda.
    return start, start + timedelta(days=43)

def meeting_link(event):
    """Keep only the join link and provider; never cache the event's description."""
    conference = event.get('conferenceData') or {}
    candidates = [(entry.get('uri', ''), True) for entry in conference.get('entryPoints', [])
                  if entry.get('entryPointType') == 'video']
    candidates.append((event.get('hangoutLink', ''), True))
    description = html.unescape(event.get('description', '') + ' ' + event.get('location', ''))
    candidates.extend((url.rstrip('.,;)'), False) for url in re.findall(r'https?://[^\s<>"\x27]+', description))
    providers = {'meet.google.com': 'Meet', 'zoom.us': 'Zoom', 'teams.microsoft.com': 'Teams',
                 'teams.live.com': 'Teams', 'whereby.com': 'Whereby', 'webex.com': 'Webex'}
    for url, structured in candidates:
        try:
            parsed = urlparse(url)
            host = (parsed.hostname or '').lower()
            provider = next((name for domain, name in providers.items()
                             if host == domain or host.endswith('.' + domain)), None)
            if parsed.scheme == 'https' and host and not parsed.username and (structured or provider):
                return url, provider or conference.get('conferenceSolution', {}).get('name', 'Meeting')
        except ValueError:
            continue
    return None, None

def sync(month, expected_account):
    from credential_store import load_credentials
    from googleapiclient.discovery import build
    from google_auth_httplib2 import AuthorizedHttp
    import httplib2
    credentials, account = load_credentials()
    if not expected_account or account != expected_account:
        raise RuntimeError('Calendar account does not match the configured account')
    start, end = calendar_window(month)
    api = build('calendar', 'v3', http=AuthorizedHttp(credentials, http=httplib2.Http(timeout=20)), cache_discovery=False)
    events = []
    token = None
    while True:
        result = api.events().list(calendarId=account, singleEvents=True, orderBy='startTime',
            timeMin=datetime.combine(start, datetime.min.time()).astimezone().isoformat(),
            timeMax=datetime.combine(end, datetime.min.time()).astimezone().isoformat(),
            maxResults=250, pageToken=token,
            fields='nextPageToken,items(id,summary,start,end,status,eventType,attendees(self,responseStatus),htmlLink,hangoutLink,conferenceData(entryPoints(entryPointType,uri),conferenceSolution(name)),description,location)').execute()
        for event in result.get('items', []):
            if event.get('status') == 'cancelled' or event.get('eventType') in ('outOfOffice', 'workingLocation'):
                continue
            if any(a.get('self') and a.get('responseStatus') == 'declined' for a in event.get('attendees', [])):
                continue
            begin, finish = event.get('start', {}), event.get('end', {})
            if not (begin.get('dateTime') or begin.get('date')):
                continue
            join_url, provider = meeting_link(event)
            events.append({'id': event['id'], 'title': event.get('summary', 'Untitled event'),
                'start': begin.get('dateTime') or begin['date'],
                'end': finish.get('dateTime') or finish.get('date') or begin.get('dateTime') or begin['date'],
                'all_day': 'date' in begin, 'url': event.get('htmlLink', ''),
                'meeting_url': join_url, 'meeting_provider': provider})
        token = result.get('nextPageToken')
        if not token:
            break
    data = {'account': account, 'month': month,
            'updated_at': datetime.now(timezone.utc).isoformat(), 'events': events}
    return data

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--month', default=datetime.now().astimezone().strftime('%Y-%m'))
    parser.add_argument('--account', required=True)
    args = parser.parse_args()
    try:
        print(json.dumps(sync(args.month, args.account)))
    except Exception as error:
        # Never put authentication responses or request URLs into the journal.
        print(json.dumps({'error': f'Calendar sync failed ({type(error).__name__})'}), file=sys.stderr)
        sys.exit(1)
