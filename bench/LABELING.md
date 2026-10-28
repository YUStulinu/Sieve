# Labelling guide

One label per message, from the point of view of the person whose inbox it is (the reader).

| Label | When |
|---|---|
| `reply_today` | A person expects an answer or an action from the reader: a direct question, a request, a decision, a date to confirm, a deadline that needs the reader's response, a problem the reader must handle. Also an account or security alert that needs action now. |
| `informative` | Worth knowing, nothing to answer: updates, confirmations (orders, bookings, deliveries, appointments), notifications, announcements, reports, FYIs, a reply that only thanks or closes a topic, mail where the reader is only in copy. |
| `newsletter` | Bulk mail sent to many people: newsletters, digests, marketing, promotions, product announcements, invitations sent to a mailing list. |
| `invoice` | Money owed or paid: invoices, bills, receipts, payment confirmations and reminders, statements, subscription charges. |

Rules for the hard cases:

1. **Decide by what the message asks of the reader, not by its topic.** A colleague's update about a
   project is `informative`, while their question about it is `reply_today`.
2. **A newsletter is never `reply_today`**, even when it says "act now".
3. **An internal announcement to the whole company** (a policy, an event, an IT notice) is
   `informative` unless it asks the reader to do something by a date. In that case it is
   `reply_today` (for example "submit your timesheet by Friday").
4. **Money with a personal question** (a client disputing an invoice, a colleague asking you to
   approve an expense) is `reply_today`. The bill itself is `invoice`.
5. **A request addressed to a long list** where the reader is one of many is `informative` unless the
   reader is clearly expected to act.
6. When unsure between `reply_today` and `informative`, choose `reply_today` only if not answering
   would let someone down.

The labels in `bench/` were assigned by the project's author following this guide, not by the
mailbox owners. Your own labels, from the app's Label mode, measure the triage on your mail.
