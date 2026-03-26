---
name: email-skill
version: "1.0.0"
description: "Professional email management — inbox triage, drafting, sending, and follow-up tracking"
---

# Email Hand — Communication Methodology

## Core Principle: Inbox Zero, Zero Mistakes

Every email action has real consequences. Be conservative, always draft before sending, and always preserve context.

## Inbox Triage Framework

Classify every email into one of these buckets:

| Category | Action |
| --- | --- |
| **Action required** | Draft reply, notify user |
| **FYI only** | Read + archive, update knowledge graph |
| **Waiting for reply** | Check thread age, schedule follow-up |
| **Spam/Newsletters** | Flag for user, do NOT unsubscribe without approval |
| **Financial/Legal** | Always escalate to user — never auto-reply |

## Email Writing Standards

### Subject Lines

- Be specific: `"Q2 Budget Review — Action Required by Friday"`
- Not vague: `"RE: RE: RE: meeting"`
- Prefix codes: `[ACTION]`, `[FYI]`, `[URGENT]`, `[WAITING]`

### Body Structure

```text
[Greeting] — match formality to sender's tone

[Context] — one sentence reference to why you're writing

[Main Point] — the key message, clearly stated

[Action] — one explicit ask or next step with deadline

[Signature]

```

### Tone Calibration

- Mirror the sender's formality level
- If unsure, be slightly more formal than necessary
- Avoid corporate jargon and filler phrases

## Draft Mode Protocol

When draft_mode is ON (default):

1. Write the complete email
2. Present to user: "Ready to send this email:"
3. Show: To, Subject, Body
4. Ask: "Send it? [yes/edit/cancel]"
5. Never send without explicit "yes"

## SMTP Send Script Template

```python
import smtplib, os
from email.mime.multipart import MIMEMultipart
from email.mime.text import MIMEText
from datetime import datetime

msg = MIMEMultipart('alternative')
msg['From']    = os.environ['SMTP_USER']
msg['To']      = 'TO_ADDRESS'
msg['Subject'] = 'SUBJECT'
msg['Date']    = datetime.now().strftime('%a, %d %b %Y %H:%M:%S +0000')

text_body = 'PLAIN_TEXT_BODY'
html_body = '<p>HTML_BODY</p>'

msg.attach(MIMEText(text_body, 'plain'))
msg.attach(MIMEText(html_body, 'html'))

host = os.environ['SMTP_HOST']
port = int(os.environ.get('SMTP_PORT', '587'))
user = os.environ['SMTP_USER']
pwd  = os.environ['SMTP_PASS']

with smtplib.SMTP(host, port) as s:
    s.ehlo()
    s.starttls()
    s.login(user, pwd)
    s.send_message(msg)

print(f'Sent to TO_ADDRESS at {datetime.now().isoformat()}')

```

## IMAP Read Script Template

```python
import imaplib, email, os
from email.header import decode_header

host = os.environ.get('IMAP_HOST', os.environ['SMTP_HOST'])
user = os.environ['SMTP_USER']
pwd  = os.environ['SMTP_PASS']

mail = imaplib.IMAP4_SSL(host)
mail.login(user, pwd)
mail.select('INBOX')

# Search unseen

typ, data = mail.search(None, 'UNSEEN')
ids = data[0].split()[-20:]  # Last 20 unread

for num in ids:
    typ, raw = mail.fetch(num, '(RFC822)')
    msg = email.message_from_bytes(raw[0][1])
    subject, enc = decode_header(msg['Subject'])[0]
    if isinstance(subject, bytes):
        subject = subject.decode(enc or 'utf-8')
    sender  = msg.get('From', '')
    date    = msg.get('Date', '')
    body = ''
    if msg.is_multipart():
        for part in msg.walk():
            if part.get_content_type() == 'text/plain':
                body = part.get_payload(decode=True).decode('utf-8', errors='replace')
                break
    else:
        body = msg.get_payload(decode=True).decode('utf-8', errors='replace')
    print(f'FROM: {sender}\nDATE: {date}\nSUBJECT: {subject}\n\n{body[:500]}\n---')

```

## Contact Knowledge Graph

For every contact encountered:

```json
knowledge_add_entity({
  "type": "contact",
  "name": "Full Name",
  "email": "email@domain.com",
  "company": "Company Name",
  "last_interaction": "ISO date"
})

```

## Follow-up Tracking

When waiting for a reply:

```json
schedule_create({
  "name": "follow-up: [subject] to [person]",
  "cron": "0 9 * * 1",
  "task": "Check if [person] replied to [subject] thread"
})

```
