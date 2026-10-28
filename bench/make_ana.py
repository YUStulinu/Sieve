"""Writes the rest of Ana Popescu's (fictional) mailbox, written for this evaluation:

  bench/ana_sent.jsonl   messages she sent, in her style (Romanian "Salut …, / Mersi, Ana", English "Hi …, / Best, Ana");
                         label: whether she is now waiting for an answer (for the follow-up evaluation)
  bench/threads.jsonl    six threads of four to six messages, for the summaries; bench/threads_facts.json lists,
                         per thread, the facts a summary must keep and the decisions taken
"""
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ANA = "ana.popescu@sieve.test"
DAY = 86400
T0 = 1_793_000_000  # late October 2026

sent = []


def s(waiting, lang, to, subject, body, days_ago=5):
    sent.append({"id": f"s{len(sent) + 1:03d}", "from": ANA, "from_name": "Ana Popescu", "to": [to], "cc": [], "subject": subject,
                 "body": body, "lang": lang, "label": "expects_reply" if waiting else "no_reply", "owner": ANA,
                 "date": T0 - days_ago * DAY, "attachments": [], "list_unsubscribe": False})


s(True, "ro", "mihai.ionescu@clientnord.ro", "Oferta pentru site",
  "Salut Mihai,\n\nÎți trimit atașat oferta pentru site, cu cele două variante discutate. Îmi spui până vineri care vă convine, ca să planific echipa?\n\nMersi,\nAna")
s(False, "ro", "elena.radu@studionord.ro", "Re: Prezentarea de joi",
  "Salut Elena,\n\nRămâne joi la 14, am mutat-o în calendar.\n\nMersi,\nAna")
s(True, "ro", "contabil@expertfisc.ro", "Contractul nou",
  "Salut Ioana,\n\nȚi-am trimis contractul nou și extrasele. Poți să-mi confirmi că ai tot ce-ți trebuie pentru declarație?\n\nMersi,\nAna")
s(False, "ro", "bogdan.stan@yahoo.com", "Re: Weekend la Arieșeni?",
  "Salut Bogdan,\n\nVin cu drag! Aduc eu cafeaua și cărțile de joc.\n\nMersi,\nAna")
s(True, "ro", "laura.mihai@agentiapixel.ro", "Re: Colaborare pe proiectul Banca Verde",
  "Salut Laura,\n\nÎn iunie aș putea doar două săptămâni, din 9. Tariful meu e 900 lei pe zi. Ți-ar merge așa?\n\nMersi,\nAna")
s(False, "ro", "mama@familia.ro", "Re: Duminică",
  "Salut mama,\n\nVenim amândoi, pe la 13. Aducem noi desertul.\n\nPupici,\nAna")
s(True, "ro", "dan.marinescu@gmail.com", "Chiria pe iunie",
  "Bună ziua, domnule Marinescu,\n\nVă rog să-mi spuneți dacă pot plăti chiria pe iunie cu o săptămână mai târziu, pe 12. Am o întârziere la o încasare.\n\nMulțumesc,\nAna Popescu")
s(False, "ro", "andrei.pop@clientnord.ro", "Re: Factura 2026/118",
  "Salut Andrei,\n\nAi dreptate, am inclus din greșeală și ziua de training. Atașez factura corectă, 4.200 lei.\n\nMersi,\nAna")
s(True, "ro", "tipografie@printexpress.ro", "Comandă broșuri",
  "Bună ziua,\n\nAș vrea o ofertă pentru 500 de broșuri A5, 16 pagini, hârtie mată 150 g. Cât ar costa și în cât timp ar fi gata?\n\nMulțumesc,\nAna Popescu")
s(False, "ro", "elena.radu@studionord.ro", "Fișierele pentru tipografie",
  "Salut Elena,\n\nLe-am urcat pe drive în folderul Tipar. Tot ce era de verificat am verificat.\n\nMersi,\nAna")
s(True, "ro", "it@studionord.ro", "Licență Figma",
  "Salut,\n\nLicența mea de Figma expiră săptămâna viitoare. Puteți s-o reînnoiți până joi? Altfel nu pot lucra la proiectul nou.\n\nMersi,\nAna")
s(False, "ro", "secretariat@scoala12cluj.ro", "Re: Acord excursie clasa a III-a",
  "Bună ziua,\n\nDA, Maria participă la excursie.\n\nMulțumesc,\nAna Popescu")
s(True, "en", "tom.baker@northwind.co.uk", "Brand guidelines draft",
  "Hi Tom,\n\nHere is the first draft of the brand guidelines. Could you send me your comments by Wednesday, so I can finish before the board meeting?\n\nBest,\nAna")
s(False, "en", "sara.lind@lindstudio.se", "Re: Workshop in Stockholm?",
  "Hi Sara,\n\nI'd love to! Count me in for 18-19 June.\n\nBest,\nAna")
s(True, "en", "accounts@northwind.co.uk", "Payment for SN-2026-038",
  "Hi James,\n\nInvoice SN-2026-038 was due last Friday and I haven't received the payment yet. Could you check when it will be paid?\n\nBest,\nAna")
s(False, "en", "kate.morris@brightagency.com", "Re: Can we move our call?",
  "Hi Kate,\n\nTomorrow at 10 works for me.\n\nBest,\nAna")
s(True, "en", "mark.chen@pixelforge.io", "Icon licence",
  "Hi Mark,\n\nThe icons are under our studio licence, which covers commercial use. Do you need a signed copy of the licence for legal? Let me know.\n\nBest,\nAna")
s(False, "en", "anna.k@designweek.org", "Re: Interview for Design Week",
  "Hi Anna,\n\nThanks for the questions — my answers are below. Good luck with the piece!\n\nBest,\nAna")
s(True, "en", "peter.hall@gmail.com", "Deposit",
  "Hi Peter,\n\nI left the keys in the lockbox on Friday. When can I expect the deposit back?\n\nBest,\nAna")
s(False, "en", "vlad.popescu@gmail.com", "Re: Dinner tonight?",
  "Hi,\n\nYes! 8 is perfect. See you there.\n\nBest,\nAna")
s(True, "en", "recruiting@figma.com", "Re: Next steps: design interview",
  "Hi Nina,\n\nNone of the slots this week work for me, unfortunately. Could we do Monday or Tuesday next week instead?\n\nBest,\nAna")
s(False, "en", "tom.baker@northwind.co.uk", "Logo files",
  "Hi Tom,\n\nThe CMYK logo files are attached, in all three versions.\n\nBest,\nAna")
s(True, "en", "lucas.f@brightagency.com", "Welcome + a question",
  "Hi Lucas,\n\nWelcome to Bright! I'm the freelance designer on the Northwind account. Would you have 20 minutes this week to go through the brand together?\n\nBest,\nAna")
s(False, "en", "kate.morris@brightagency.com", "Re: Contract renewal",
  "Hi Kate,\n\nYes, I'd like to renew on the same terms. Thanks for sending the paperwork.\n\nBest,\nAna")

with (HERE / "ana_sent.jsonl").open("w", encoding="utf-8", newline="\n") as f:
    for r in sent:
        f.write(json.dumps(r, ensure_ascii=False) + "\n")

# ---- threads ----
threads, facts = [], {}


def thread(tid, lang, subject, messages, keep, decided):
    for i, (frm, name, body) in enumerate(messages):
        threads.append({"id": f"{tid}-{i + 1}", "thread": tid, "from": frm, "from_name": name, "to": [ANA], "subject": ("Re: " if i else "") + subject,
                        "body": body, "lang": lang, "date": T0 + i * DAY // 2, "owner": ANA})
    facts[tid] = {"keep": keep, "decided": decided}


thread("t1", "ro", "Lansarea site-ului Client Nord", [
    ("mihai.ionescu@clientnord.ro", "Mihai Ionescu", "Bună Ana, vrem să lansăm site-ul pe 1 iunie. E realist cu paginile de produs?"),
    (ANA, "Ana Popescu", "Salut Mihai, e realist dacă primim textele până pe 15 mai. Fără texte nu putem termina machetele."),
    ("andrei.pop@clientnord.ro", "Andrei Pop", "Textele le scrie agenția noastră, dar ei pot abia pe 20 mai."),
    ("mihai.ionescu@clientnord.ro", "Mihai Ionescu", "Atunci mutăm lansarea pe 8 iunie. Ana, e ok pentru tine?"),
    (ANA, "Ana Popescu", "Da, 8 iunie e ok dacă textele vin pe 20 mai. Rămâne de stabilit cine face fotografiile de produs."),
], ["8 iunie", "20 mai"], ["lansarea se mută pe 8 iunie"])
thread("t2", "ro", "Renovarea fațadei bloc 7", [
    ("office@asociatiabloc7.ro", "Asociația bloc 7", "Avem trei oferte pentru fațadă: 410.000 lei, 465.000 lei și 520.000 lei. Votăm în ședința de joi."),
    ("dan.marinescu@gmail.com", "Dan Marinescu", "Oferta de 410.000 nu include schela. Cu schela ajunge la 470.000."),
    ("office@asociatiabloc7.ro", "Asociația bloc 7", "Corect. Propunem oferta de 465.000 lei, care include tot. Fiecare apartament ar plăti aproximativ 9.300 lei, în 12 rate."),
    ("sorin.vlad@yahoo.com", "Sorin Vlad", "Sunt de acord cu 465.000. Când începe lucrarea?"),
    ("office@asociatiabloc7.ro", "Asociația bloc 7", "Dacă votul trece joi, lucrarea începe pe 1 iulie și durează două luni."),
], ["465.000", "9.300", "1 iulie", "12 rate"], ["se propune oferta de 465.000 lei"])
thread("t3", "ro", "Excursia de la Arieșeni", [
    ("bogdan.stan@yahoo.com", "Bogdan Stan", "Am găsit două cabane: Arnica, 600 lei pe noapte, și Vârtop, 450 lei, dar fără bucătărie."),
    ("ioana.stan@yahoo.com", "Ioana Stan", "Eu aș lua Arnica, cu bucătărie gătim noi și ies mai ieftin per total."),
    (ANA, "Ana Popescu", "De acord cu Arnica. Câți suntem? Eu vin cu Vlad."),
    ("bogdan.stan@yahoo.com", "Bogdan Stan", "Suntem 6. Iau Arnica pentru 15-17 mai, două nopți, 1.200 lei, adică 200 de lei de persoană. Vă rog să-mi trimiteți banii până miercuri."),
], ["Arnica", "15-17 mai", "200 de lei", "miercuri"], ["au ales cabana Arnica"])
thread("t4", "en", "Northwind brand guidelines", [
    ("tom.baker@northwind.co.uk", "Tom Baker", "Ana, the board meets on 12 June. Can the guidelines be ready by then?"),
    (ANA, "Ana Popescu", "Yes, if I get your comments on the draft by 3 June."),
    ("tom.baker@northwind.co.uk", "Tom Baker", "Comments attached. The main one: the board wants a photography section."),
    ("lucy.grant@northwind.co.uk", "Lucy Grant", "For photography we have a budget of £3,000 for a shoot, if Ana can art-direct it."),
    (ANA, "Ana Popescu", "Happy to art-direct. The shoot would need to happen by 6 June to make the 12 June deadline."),
    ("tom.baker@northwind.co.uk", "Tom Baker", "Great — Lucy will book the photographer for 5 June."),
], ["12 June", "£3,000", "5 June"], ["a photography section is added", "the shoot is on 5 June"])
thread("t5", "en", "Workshop logistics", [
    ("sara.lind@lindstudio.se", "Sara Lind", "Hi all, the workshop is at Fotografiska, 18-19 June. Each speaker has 90 minutes."),
    ("mark.chen@pixelforge.io", "Mark Chen", "Will there be a projector with HDMI? I'll bring my own laptop."),
    ("sara.lind@lindstudio.se", "Sara Lind", "Yes, HDMI and USB-C. We cover flights up to €300 and two hotel nights."),
    (ANA, "Ana Popescu", "My flight is €340. Is the difference ok, or should I look for a cheaper one?"),
    ("sara.lind@lindstudio.se", "Sara Lind", "Ana, €340 is fine — we'll cover it."),
], ["Fotografiska", "18-19 June", "90 minutes", "€340"], ["Ana's €340 flight is covered"])
thread("t6", "en", "Invoice dispute SN-2026-041", [
    ("accounts@northwind.co.uk", "James", "Invoice SN-2026-041 lists 12 days, our PO covers 10."),
    (ANA, "Ana Popescu", "The two extra days were the photo shoot, which Tom approved by email on 2 June."),
    ("tom.baker@northwind.co.uk", "Tom Baker", "Confirmed, I approved the two extra days. James, please pay all 12."),
    ("accounts@northwind.co.uk", "James", "Thanks Tom. We'll raise the PO to 12 days and pay in the next run, on 30 June."),
], ["12 days", "30 June", "Tom"], ["all 12 days will be paid"])

with (HERE / "threads.jsonl").open("w", encoding="utf-8", newline="\n") as f:
    for r in threads:
        f.write(json.dumps(r, ensure_ascii=False) + "\n")
(HERE / "threads_facts.json").write_text(json.dumps(facts, ensure_ascii=False, indent=1), encoding="utf-8")
print(len(sent), "sent;", len(threads), "thread messages in", len(facts), "threads")
