"""Writes bench/triage_written.jsonl: messages written for this evaluation, for the inbox of Ana Popescu, a designer in Cluj
(fictional). Romanian and English, 15 per language and pile, with the hard cases of bench/LABELING.md."""
import json
from pathlib import Path

ANA = "ana.popescu@sieve.test"
rows = []


def m(label, lang, frm, name, subject, body, to=None, cc=None, unsub=False, att=None):
    rows.append({
        "id": f"w{len(rows) + 1:03d}", "from": frm, "from_name": name, "to": to if to is not None else [ANA], "cc": cc or [],
        "subject": subject, "body": body, "attachments": att or [], "list_unsubscribe": unsub, "lang": lang, "label": label,
        "owner": ANA, "source": "written",
    })


# ---- Romanian: reply_today ----
m("reply_today", "ro", "mihai.ionescu@clientnord.ro", "Mihai Ionescu", "Oferta pentru site",
  "Bună Ana,\n\nAm discutat intern și am vrea să mergem mai departe cu varianta a doua. Poți să-mi trimiți oferta finală până mâine la prânz? Avem ședință de buget după-amiază.\n\nMulțumesc,\nMihai")
m("reply_today", "ro", "elena.radu@studionord.ro", "Elena Radu", "Prezentarea de joi",
  "Ana, ne vedem joi la 10 sau mutăm la 14? Clientul a cerut o oră mai târzie. Spune-mi azi ca să le confirm.\n\nElena")
m("reply_today", "ro", "proprietar.ap12@gmail.com", "Dan Marinescu", "Vizită contor apă",
  "Bună ziua,\n\nVine cineva de la Apă Canal să citească contoarele sâmbătă între 9 și 12. Puteți fi acasă? Dacă nu, îmi lăsați cheia la vecina de la 13.\n\nDan Marinescu")
m("reply_today", "ro", "contabil@expertfisc.ro", "Ioana Vlad", "Documente pentru declarația unică",
  "Bună Ana,\n\nPentru declarația unică am nevoie de extrasele de cont pe decembrie și de contractul nou cu Studio Nord. Mi le poți trimite până vineri? După aceea nu mai apuc să o depun la termen.\n\nIoana")
m("reply_today", "ro", "secretariat@scoala12cluj.ro", "Secretariat Școala 12", "Acord excursie clasa a III-a",
  "Stimați părinți,\n\nVă rugăm să confirmați până miercuri dacă fiica dumneavoastră, Maria, participă la excursia de la Turda din 14 mai. Răspundeți la acest email cu DA sau NU.\n\nSecretariatul")
m("reply_today", "ro", "andrei.pop@clientnord.ro", "Andrei Pop", "Re: Factura 2026/118",
  "Bună Ana,\n\nAm primit factura, dar suma nu corespunde cu oferta: în ofertă erau 4.200 lei, pe factură 4.800. Poți să verifici și să-mi spui de unde e diferența? Până nu clarificăm nu o pot trimite la plată.\n\nAndrei", att=[])
m("reply_today", "ro", "securitate@bancatransilvania.ro", "Banca Transilvania", "Tranzacție neautorizată? Confirmați acum",
  "Am blocat temporar cardul dumneavoastră terminat în 4821 după o tentativă de plată de 1.250 lei la un comerciant din străinătate. Dacă nu ați inițiat-o, sunați imediat la 0264 308 028. Dacă ați inițiat-o, confirmați în aplicație pentru deblocare.")
m("reply_today", "ro", "hr@studionord.ro", "HR Studio Nord", "Pontaj aprilie — termen azi",
  "Bună tuturor,\n\nVă rog completați pontajul pe aprilie în aplicație până azi la ora 17. Fără pontaj nu putem procesa salariile la timp.\n\nMulțumesc,\nCristina", to=["echipa@studionord.ro"])
m("reply_today", "ro", "bogdan.stan@yahoo.com", "Bogdan Stan", "Weekend la Arieșeni?",
  "Salut Ana! Mergem la Arieșeni în weekendul ăsta, plecăm vineri seara. Mai e un loc în mașină. Vii? Spune-mi până diseară că trebuie să confirm cabana.")
m("reply_today", "ro", "clinica@dentalcare.ro", "Dental Care", "Confirmare programare",
  "Bună ziua,\n\nAveți programare mâine, 12 mai, la ora 9:30 la dr. Muntean. Vă rugăm să confirmați prezența răspunzând la acest mesaj sau să ne anunțați dacă doriți reprogramare.")
m("reply_today", "ro", "laura.mihai@agentiapixel.ro", "Laura Mihai", "Colaborare pe proiectul Banca Verde",
  "Bună Ana,\n\nAvem un proiect de rebranding pentru o bancă și am nevoie de un designer senior pentru 3 săptămâni din iunie. Ai fi disponibilă? Dacă da, ce tarif ai pe zi?\n\nLaura")
m("reply_today", "ro", "mihai.ionescu@clientnord.ro", "Mihai Ionescu", "Re: Machete homepage",
  "Mulțumesc pentru machete. Două întrebări: logo-ul poate fi mai mic în header? Și putem vedea și o variantă cu fundal deschis? Aș vrea să le arăt directorului mâine dimineață.")
m("reply_today", "ro", "sorina.popescu@gmail.com", "Mama", "Duminică",
  "Ana, vii duminică la masă cu Vlad? Să știu câtă friptură să fac. Pupici!")
m("reply_today", "ro", "office@asociatiabloc7.ro", "Asociația de proprietari bloc 7", "Vot renovare fațadă — până vineri",
  "Stimați proprietari,\n\nPentru a depune dosarul de reabilitare termică avem nevoie de acordul fiecărui proprietar. Vă rugăm să semnați formularul atașat și să-l lăsați la administrator până vineri.", to=["proprietari@asociatiabloc7.ro"], att=["acord_reabilitare.pdf"])
m("reply_today", "ro", "elena.radu@studionord.ro", "Elena Radu", "Urgent: fișierele pentru tipografie",
  "Ana, tipografia așteaptă fișierele pentru broșură și închide la 16. Le poți urca tu pe drive în următoarea oră? Eu sunt la client și nu am acces.")

# ---- Romanian: informative ----
m("informative", "ro", "noreply@emag.ro", "eMAG", "Comanda ta 41827733 a fost livrată",
  "Salut, Ana!\n\nComanda ta a fost livrată la adresa din Str. Horea 22. Mulțumim că ai cumpărat de la eMAG!", to=[ANA])
m("informative", "ro", "noreply@fancourier.ro", "FAN Courier", "AWB 2400345671 — coletul tău ajunge mâine",
  "Coletul expediat de Libris SRL va fi livrat mâine, 13 mai, între 10:00 și 14:00. Poți urmări livrarea pe fancourier.ro.")
m("informative", "ro", "elena.radu@studionord.ro", "Elena Radu", "Re: Fișierele pentru tipografie",
  "Gata, le-am primit, mulțumesc! Le trimit acum la tipografie.")
m("informative", "ro", "andrei.pop@clientnord.ro", "Andrei Pop", "Plan proiect actualizat",
  "Bună tuturor,\n\nAtașez planul de proiect actualizat. Lansarea rămâne pe 1 iunie; am mutat doar testarea cu o săptămână mai devreme.\n\nAndrei", to=["mihai.ionescu@clientnord.ro"], cc=[ANA], att=["plan_proiect_v4.pdf"])
m("informative", "ro", "mihai.ionescu@clientnord.ro", "Mihai Ionescu", "Re: Oferta pentru site",
  "Am primit oferta, mulțumesc. Revin săptămâna viitoare după ședința de buget.")
m("informative", "ro", "programari@dentalcare.ro", "Dental Care", "Programarea dvs. a fost confirmată",
  "Programarea din 12 mai, ora 9:30, la dr. Muntean a fost confirmată. Vă așteptăm cu 10 minute înainte.")
m("informative", "ro", "hr@studionord.ro", "HR Studio Nord", "Program de sărbători",
  "Bună tuturor,\n\nVă reamintim că pe 1 iunie birourile sunt închise. Program normal din 2 iunie.\n\nCristina", to=["echipa@studionord.ro"])
m("invoice", "ro", "notificari@ghiseul.ro", "Ghiseul.ro", "Plata a fost înregistrată",
  "Plata dumneavoastră către Primăria Cluj-Napoca a fost înregistrată cu succes. Nu este necesară nicio altă acțiune.")
m("informative", "ro", "bogdan.stan@yahoo.com", "Bogdan Stan", "Poze de la Arieșeni",
  "Am pus pozele de la Arieșeni aici: drive.google.com/… Mai pun și filmulețele diseară.")
m("informative", "ro", "it@studionord.ro", "IT Studio Nord", "Mentenanță server sâmbătă",
  "Sâmbătă între 8 și 12 serverul de fișiere nu va fi disponibil pentru actualizări. Salvați local ce aveți nevoie.", to=["echipa@studionord.ro"])
m("informative", "ro", "no-reply@booking.com", "Booking.com", "Rezervarea ta la Pensiunea Arnica este confirmată",
  "Rezervare confirmată: Pensiunea Arnica, Arieșeni, 15–17 mai, 2 nopți. Check-in de la ora 14:00. Plata se face la proprietate.")
m("informative", "ro", "dan.marinescu@gmail.com", "Dan Marinescu", "Re: Vizită contor apă",
  "În regulă, las cheia la vecina de la 13. Mulțumesc!")
m("informative", "ro", "secretariat@scoala12cluj.ro", "Secretariat Școala 12", "Rezultate concurs de desen",
  "Felicitări tuturor elevilor participanți! Rezultatele concursului de desen sunt afișate la avizier. Premierea are loc pe 20 mai.", to=["parinti-3b@scoala12cluj.ro"])
m("informative", "ro", "noreply@anaf.ro", "ANAF", "Ați primit un mesaj în Spațiul Privat Virtual",
  "Aveți un document nou în Spațiul Privat Virtual: Recipisa declarației unice. Accesați anaf.ro pentru a-l vizualiza.")
m("informative", "ro", "laura.mihai@agentiapixel.ro", "Laura Mihai", "Re: Colaborare pe proiectul Banca Verde",
  "Mulțumesc pentru răspuns! Înțeleg că în iunie ești ocupată. Te țin minte pentru proiectele din toamnă.")

# ---- Romanian: newsletter ----
m("newsletter", "ro", "newsletter@libris.ro", "Libris", "Cărțile lunii mai — reduceri de până la 40%",
  "Descoperă noutățile lunii: romane, cărți pentru copii și albume de artă, cu reduceri de până la 40%. Oferta e valabilă până duminică.", unsub=True)
m("newsletter", "ro", "noutati@emag.ro", "eMAG", "Doar azi: Black Friday de primăvară!",
  "Răspunde rapid! Mii de produse cu prețuri reduse doar azi. Nu rata ofertele de electrocasnice și laptopuri.", unsub=True)
m("newsletter", "ro", "newsletter@dor.ro", "Revista DoR", "Scrisoarea de duminică: despre orașe și oameni",
  "Bună dimineața! În numărul de săptămâna asta: o poveste despre cartierele noi ale Clujului, un interviu cu un arhitect și recomandări de lectură.", unsub=True)
m("newsletter", "ro", "comunitate@designromania.ro", "Design România", "Invitație: Seara designerilor, 22 mai",
  "Te invităm la Seara designerilor, joi 22 mai, la Fabrica de Pensule. Prezentări, networking și expoziție. Intrarea e liberă, cu înscriere.", unsub=True, to=["membri@designromania.ro"])
m("newsletter", "ro", "promo@orange.ro", "Orange", "Abonamente noi cu internet nelimitat",
  "Treci acum la noile abonamente Orange cu internet nelimitat și primești 3 luni de Netflix gratuit. Vezi ofertele în magazinele Orange.", unsub=True)
m("newsletter", "ro", "info@decathlon.ro", "Decathlon", "Echipează-te pentru drumeții",
  "Sezonul de munte a început! Rucsaci, bocanci și echipament de camping cu 20% reducere pentru membri.", unsub=True)
m("newsletter", "ro", "news@bancatransilvania.ro", "Banca Transilvania", "Noutăți BT: economisește mai ușor",
  "Află cum poți economisi automat cu noile funcții din BT Pay. Plus: dobânzi mai bune la depozitele de 6 luni.", unsub=True)
m("newsletter", "ro", "newsletter@scena9.ro", "Scena9", "Ce vedem săptămâna asta",
  "Recomandările noastre culturale: un film românesc nou, două expoziții la Cluj și un festival de jazz.", unsub=True)
m("newsletter", "ro", "noreply@linkedin.com", "LinkedIn", "Ana, ai 5 noi vizualizări ale profilului",
  "Află cine ți-a vizitat profilul săptămâna asta și descoperă joburi recomandate pentru tine.", unsub=True)
m("newsletter", "ro", "contact@cafeneauaboema.ro", "Boema", "Meniu nou de primăvară",
  "Dragi prieteni, avem meniu nou de primăvară! Vă așteptăm cu limonade de casă și deserturi cu rubarbă. Răspundeți cu „DA” pentru o rezervare la terasă!", unsub=True)
m("newsletter", "ro", "webinar@academiaonline.ro", "Academia Online", "Webinar gratuit: Figma pentru profesioniști",
  "Înscrie-te la webinarul gratuit de marți, ora 19. Locurile sunt limitate — rezervă-ți locul acum!", unsub=True)
m("newsletter", "ro", "newsletter@hotnews.ro", "HotNews", "Știrile dimineții",
  "Principalele știri de azi: guvernul discută bugetul, prețul energiei scade, vreme frumoasă în weekend.", unsub=True)
m("newsletter", "ro", "oferte@flanco.ro", "Flanco", "Ultima zi: rate fără dobândă",
  "Azi e ultima zi pentru rate fără dobândă la televizoare și laptopuri. Comandă online cu livrare gratuită.", unsub=True)
m("newsletter", "ro", "noreply@facebook.com", "Facebook", "Ai 12 notificări noi",
  "Bogdan Stan și alți 4 prieteni au postat fotografii noi. Vezi ce ai ratat.", unsub=True)
m("newsletter", "ro", "comunitate@designromania.ro", "Design România", "Buletinul lunar al comunității",
  "Ce s-a întâmplat luna asta: concursuri, joburi noi în industrie și un ghid despre tipografie românească.", unsub=True, to=["membri@designromania.ro"])

# ---- Romanian: invoice ----
m("invoice", "ro", "facturi@enel.ro", "Enel", "Factura ta Enel pentru aprilie",
  "Factura pentru energie electrică pe aprilie este disponibilă: 184,30 lei, scadentă pe 25 mai. O poți plăti în aplicația Enel sau la bancă.", att=["factura_enel_aprilie.pdf"])
m("invoice", "ro", "facturare@digi.ro", "Digi", "Factura Digi nr. 88412093",
  "Stimate client, factura pentru serviciile Digi din luna mai, în valoare de 67,00 lei, a fost emisă. Termen de plată: 20 mai.", att=["factura_digi.pdf"])
m("invoice", "ro", "noreply@orange.ro", "Orange", "Factura ta Orange e gata",
  "Factura ta Orange pentru luna mai este de 54,20 lei. Scadența este 15 iunie. Plătește simplu în aplicația My Orange.")
m("invoice", "ro", "office@asociatiabloc7.ro", "Asociația de proprietari bloc 7", "Lista de întreținere — mai",
  "Lista de întreținere pe luna mai a fost afișată. Pentru apartamentul 12 aveți de plată 312 lei, până la data de 25.", to=["proprietari@asociatiabloc7.ro"], att=["intretinere_mai.pdf"])
m("invoice", "ro", "noreply@bancatransilvania.ro", "Banca Transilvania", "Extrasul de cont pentru aprilie",
  "Extrasul de cont pentru luna aprilie este disponibil în BT24. Sold final: 8.430,12 lei.")
m("invoice", "ro", "plati@ebrd-leasing.ro", "Leasing Auto", "Reamintire: rata 14 scadentă pe 20 mai",
  "Vă reamintim că rata nr. 14 a contractului de leasing, în valoare de 1.140 lei, este scadentă pe 20 mai.")
m("invoice", "ro", "contabilitate@studionord.ro", "Contabilitate Studio Nord", "Fluturaș salariu aprilie",
  "Atașat găsiți fluturașul de salariu pentru aprilie. Plata a fost efectuată astăzi.", att=["fluturas_aprilie.pdf"])
m("invoice", "ro", "noreply@spotify.com", "Spotify", "Chitanța ta Spotify",
  "Mulțumim! Am încasat 26,99 lei pentru abonamentul Premium Family, perioada 12 mai – 11 iunie.")
m("invoice", "ro", "facturi@eon.ro", "E.ON", "Factură gaze naturale — mai",
  "Factura E.ON pentru gaze naturale în valoare de 96,45 lei a fost emisă. Scadență: 30 mai.", att=["factura_eon.pdf"])
m("invoice", "ro", "noreply@anaf.ro", "ANAF", "Notificare plată impozit",
  "Vă informăm că suma de 1.240 lei reprezentând contribuții sociale a fost înregistrată în contul dumneavoastră fiscal.")
m("invoice", "ro", "noreply@emag.ro", "eMAG", "Factura pentru comanda 41827733",
  "Factura fiscală pentru comanda ta a fost emisă. Total: 349,99 lei, plătit cu cardul.", att=["factura_41827733.pdf"])
m("invoice", "ro", "office@expertfisc.ro", "Expert Fisc", "Factura servicii contabilitate — mai",
  "Bună Ana, atașez factura pentru serviciile de contabilitate din mai, 450 lei, cu plata până la sfârșitul lunii.\n\nIoana", att=["EF_2026_0512.pdf"])
m("invoice", "ro", "noreply@uber.com", "Uber", "Cursa ta de joi seara",
  "Mulțumim că ai călătorit cu Uber. Total: 23,40 lei, achitat cu Visa •••• 4821.")
m("invoice", "ro", "plati@ghiseul.ro", "Ghiseul.ro", "Aveți obligații de plată",
  "Pentru contul dumneavoastră figurează obligații de plată către Primăria Cluj-Napoca: impozit auto 210 lei, scadent 30 septembrie.")
m("invoice", "ro", "facturi@apacanal.ro", "Apă Canal", "Factura apă — aprilie",
  "Factura pentru consumul de apă din aprilie: 78,10 lei. Scadență 28 mai.")

# ---- English: reply_today ----
m("reply_today", "en", "tom.baker@northwind.co.uk", "Tom Baker", "Logo files for the print run",
  "Hi Ana,\n\nThe printer needs the final logo files in CMYK by 3pm today. Could you send them over? Otherwise we miss this week's slot.\n\nThanks,\nTom")
m("reply_today", "en", "sara.lind@lindstudio.se", "Sara Lind", "Workshop in Stockholm?",
  "Hi Ana,\n\nWe're running a two-day branding workshop in Stockholm on 18-19 June and would love you to lead one session. Are you interested? I need to confirm speakers by Friday.\n\nBest,\nSara")
m("reply_today", "en", "accounts@northwind.co.uk", "Northwind Accounts", "Query on invoice SN-2026-041",
  "Hello,\n\nWe've received invoice SN-2026-041 but it lists 12 days of work, while our purchase order covers 10. Could you confirm which is correct, or send a corrected invoice? We can't process the payment until then.\n\nKind regards,\nJames, Accounts Payable")
m("reply_today", "en", "security@google.com", "Google", "Critical security alert",
  "Someone just used your password to try to sign in to your account from a new device in Lagos, Nigeria. Google blocked them, but you should check what happened and change your password now.")
m("reply_today", "en", "kate.morris@brightagency.com", "Kate Morris", "Can we move our call?",
  "Hi Ana, something came up — could we move today's 4pm call to tomorrow at 10? Let me know and I'll send a new invite.")
m("reply_today", "en", "recruiting@figma.com", "Figma Recruiting", "Next steps: design interview",
  "Hi Ana,\n\nThanks for applying for the Senior Product Designer role. We'd like to schedule a 45-minute portfolio review. Please pick a slot from the link below this week.\n\nBest,\nNina")
m("reply_today", "en", "landlord.uk@gmail.com", "Peter Hall", "Keys for the flat",
  "Hi Ana, the new tenants arrive Saturday morning. Can you leave the keys in the lockbox by Friday evening? Please confirm so I can tell them.")
m("reply_today", "en", "tom.baker@northwind.co.uk", "Tom Baker", "Re: Brand guidelines draft",
  "Thanks Ana, looks great overall. Two questions before I share it with the board tomorrow: can we add a section on photography, and is the secondary palette final?")
m("reply_today", "en", "visa-appointments@ukvisas.gov", "UK Visas", "Action required: upload your documents",
  "Your visa application is incomplete. Upload your bank statements and proof of employment within 48 hours, or your appointment will be cancelled.")
m("reply_today", "en", "ops@brightagency.com", "Bright Agency Ops", "Timesheets due today",
  "Hi all — please submit your timesheets for May by end of day today so we can invoice clients tomorrow. Thanks!", to=["freelancers@brightagency.com"])
m("reply_today", "en", "mark.chen@pixelforge.io", "Mark Chen", "Quick question about the icon set",
  "Hey Ana, are the icons in the set you sent licensed for commercial use? Legal is asking before we ship on Monday.")
m("reply_today", "en", "anna.k@designweek.org", "Anna Kowalski", "Interview for Design Week",
  "Dear Ana,\n\nI'm writing a piece on Eastern European studios for Design Week and would love a short interview. Would you have 20 minutes this week? My deadline is Thursday.\n\nAnna")
m("reply_today", "en", "dpo@cloudbox.com", "CloudBox", "Your storage is full — files will stop syncing",
  "Your CloudBox storage is 100% full. New files are no longer being synced. Upgrade or free up space today to avoid losing changes.")
m("reply_today", "en", "kate.morris@brightagency.com", "Kate Morris", "Contract renewal",
  "Hi Ana, our contract ends on the 31st. Would you like to renew for another six months on the same terms? If yes, I'll send the paperwork this week.")
m("reply_today", "en", "vlad.popescu@gmail.com", "Vlad", "Dinner tonight?",
  "Hey, want to try that new Thai place tonight? I can book for 8. Let me know before 5!")

# ---- English: informative ----
m("informative", "en", "no-reply@amazon.co.uk", "Amazon", "Your order has shipped",
  "Your package with 'Grid Systems in Graphic Design' has shipped and will arrive on Thursday.")
m("informative", "en", "tom.baker@northwind.co.uk", "Tom Baker", "Re: Logo files for the print run",
  "Got them, thanks Ana! All good.")
m("informative", "en", "noreply@github.com", "GitHub", "[studionord/site] Pull request merged",
  "Mark Chen merged pull request #42 'Update homepage hero' into main.")
m("informative", "en", "sara.lind@lindstudio.se", "Sara Lind", "Workshop schedule",
  "Hi everyone,\n\nAttached is the final schedule for the June workshop. Your session is on day two at 10:00. Looking forward to it!\n\nSara", to=["speakers@lindstudio.se"], att=["workshop_schedule.pdf"])
m("informative", "en", "calendar-notification@google.com", "Google Calendar", "Accepted: Brand review @ Thu 10am",
  "Kate Morris has accepted this invitation. Brand review, Thursday 10:00–11:00 (EET).")
m("informative", "en", "mark.chen@pixelforge.io", "Mark Chen", "Release notes v2.3",
  "Hi team, v2.3 is live: new onboarding, faster exports, and the dark theme. Full notes attached.", to=["team@pixelforge.io"], cc=[ANA])
m("informative", "en", "noreply@zoom.us", "Zoom", "Cloud recording is now available",
  "The cloud recording of 'Brand guidelines review' is now available. It will be kept for 30 days.")
m("informative", "en", "kate.morris@brightagency.com", "Kate Morris", "Re: Can we move our call?",
  "Perfect, tomorrow at 10 it is. Invite sent.")
m("informative", "en", "no-reply@airbnb.com", "Airbnb", "Your reservation is confirmed",
  "You're going to Stockholm! Your stay at 'Bright loft near Södermalm' is confirmed for 17–20 June. Check-in after 3pm.")
m("informative", "en", "it@brightagency.com", "Bright Agency IT", "VPN maintenance on Sunday",
  "The VPN will be unavailable on Sunday from 6 to 8 am for maintenance. No action needed.", to=["all@brightagency.com"])
m("informative", "en", "james@northwind.co.uk", "James", "Re: Query on invoice SN-2026-041",
  "Thanks for the corrected invoice. It has been approved and will be paid in the next payment run.")
m("informative", "en", "noreply@dropbox.com", "Dropbox", "Tom Baker shared 'Print proofs' with you",
  "Tom Baker shared a folder with you: Print proofs. Open it on Dropbox.")
m("informative", "en", "anna.k@designweek.org", "Anna Kowalski", "The article is out",
  "Hi Ana, the piece is published today — thanks again for your time! Link: designweek.org/eastern-studios")
m("informative", "en", "team@brightagency.com", "Bright Agency", "Welcome our new art director",
  "Please join us in welcoming Lucas Ferreira, who joins as Art Director on Monday.", to=["all@brightagency.com"])
m("informative", "en", "noreply@trainline.com", "Trainline", "Your e-ticket",
  "Your e-ticket for London Paddington → Bristol Temple Meads on 4 June is attached. Show it on your phone.", att=["eticket.pdf"])

# ---- English: newsletter ----
m("newsletter", "en", "hello@itsnicethat.com", "It's Nice That", "The week in creativity",
  "This week: a studio that designs with clay, the best posters of the month, and five new typefaces to try.", unsub=True)
m("newsletter", "en", "news@figma.com", "Figma", "Config 2026: early bird tickets",
  "Config is back! Early bird tickets are available until Friday. Join thousands of designers in San Francisco. Act now!", unsub=True)
m("newsletter", "en", "deals@booking.com", "Booking.com", "Ana, your next trip is waiting",
  "Save 15% on stays in Lisbon, Vienna and Prague. Limited-time Genius deals — book today!", unsub=True)
m("newsletter", "en", "newsletter@smashingmagazine.com", "Smashing Magazine", "Smashing Newsletter #512",
  "In this issue: accessible colour palettes, a new CSS feature, and job openings at great companies.", unsub=True)
m("newsletter", "en", "digest@medium.com", "Medium Daily Digest", "Why great designers write",
  "Today's highlights: 'Why great designers write', 'The death of the hamburger menu', and more stories picked for you.", unsub=True)
m("newsletter", "en", "promo@adobe.com", "Adobe", "40% off Creative Cloud — ends tonight",
  "Last chance: get 40% off Creative Cloud All Apps for your first year. Offer ends at midnight. Reply to this email with questions!", unsub=True)
m("newsletter", "en", "events@designmuseum.org", "Design Museum", "Members' preview: Future Cities",
  "Join us for the members' preview of 'Future Cities' on Thursday evening. RSVP on our website.", unsub=True)
m("newsletter", "en", "noreply@dribbble.com", "Dribbble", "Popular shots this week",
  "See the most popular shots of the week and discover new designers to follow.", unsub=True)
m("newsletter", "en", "hello@substack.com", "Type Notes", "Type Notes #88: variable fonts in print",
  "Hi friends, this week I'm writing about variable fonts in print and why they matter. Thanks for reading!", unsub=True)
m("newsletter", "en", "news@economist.com", "The Economist", "The world this week",
  "A summary of the week's main events in politics and business.", unsub=True)
m("newsletter", "en", "offers@ikea.com", "IKEA", "New for summer: outdoor furniture",
  "Make the most of the sun with our new outdoor collection. Members get free delivery this weekend.", unsub=True)
m("newsletter", "en", "community@behance.net", "Behance", "You've been featured in a curated gallery!",
  "Congrats! Projects like yours appear in our Branding gallery this week. Discover more work from the community.", unsub=True)
m("newsletter", "en", "webinars@hubspot.com", "HubSpot", "Webinar invite: AI for small agencies",
  "Join our free webinar next Tuesday and learn how small agencies use AI to win more clients. Save your seat!", unsub=True)
m("newsletter", "en", "noreply@linkedin.com", "LinkedIn", "Jobs you may be interested in",
  "Senior Brand Designer at Spotify, Art Director at Wise, and 8 more jobs that match your profile.", unsub=True)
m("newsletter", "en", "news@ycombinator.com", "Hacker Newsletter", "Hacker Newsletter #701",
  "The best articles from Hacker News this week, hand-picked for you.", unsub=True)

# ---- English: invoice ----
m("invoice", "en", "billing@figma.com", "Figma", "Your Figma receipt",
  "Thanks for your payment of $15.00 for Figma Professional (monthly). Receipt #F-2026-88231 is attached.", att=["receipt_F-2026-88231.pdf"])
m("invoice", "en", "invoices@brightagency.com", "Bright Agency", "Remittance advice — payment sent",
  "We have sent a payment of £2,400.00 for invoice SN-2026-038. It should reach your account within 3 working days.")
m("invoice", "en", "billing@adobe.com", "Adobe", "Your Adobe invoice",
  "Your Creative Cloud subscription has been renewed. Amount charged: €62.99. Invoice attached.", att=["adobe_invoice.pdf"])
m("invoice", "en", "noreply@paypal.com", "PayPal", "You sent a payment",
  "You sent €45.00 EUR to Type Foundry Ltd for 'Grotesk licence'. Transaction ID 7HG23451K.")
m("invoice", "en", "billing@cloudbox.com", "CloudBox", "Payment failed — update your card",
  "We couldn't charge your card for CloudBox Plus (€9.99). We'll try again in 3 days. Update your payment details to keep your plan.")
m("invoice", "en", "accounts@northwind.co.uk", "Northwind Accounts", "Statement of account — May",
  "Please find attached your statement of account for May. Outstanding balance: £0.00.", att=["statement_may.pdf"])
m("invoice", "en", "no-reply@amazon.co.uk", "Amazon", "Your Amazon.co.uk order receipt",
  "Order total: £34.99, paid with Visa ending 4821. Thank you for shopping with us.")
m("invoice", "en", "billing@notion.so", "Notion", "Your Notion invoice for May",
  "Invoice #N-48211 for Notion Plus: $10.00, paid on 1 May.", att=["notion_invoice.pdf"])
m("invoice", "en", "receipts@uber.com", "Uber", "Your Thursday evening trip with Uber",
  "Total £18.40, charged to Visa •••• 4821. Thanks for riding.")
m("invoice", "en", "billing@hetzner.com", "Hetzner", "Invoice R0021548834",
  "Your invoice R0021548834 for May is available: €8.21 for server CX22. It will be charged to your card.")
m("invoice", "en", "noreply@stripe.com", "Lind Studio via Stripe", "Receipt from Lind Studio",
  "Receipt #1932-4410: Workshop speaker fee — paid €600.00 to your account.")
m("invoice", "en", "billing@github.com", "GitHub", "[GitHub] Payment receipt for studionord",
  "We received payment for your GitHub Team plan: $48.00. Thank you!")
m("invoice", "en", "noreply@trainline.com", "Trainline", "Your booking receipt",
  "Booking reference TL9921: £62.30 paid for London → Bristol return.")
m("invoice", "en", "invoices@typefoundry.co", "Type Foundry", "Invoice TF-1123 — Grotesk desktop licence",
  "Please find attached invoice TF-1123 for the Grotesk desktop licence (5 seats): €225, due within 14 days.", att=["TF-1123.pdf"])
m("invoice", "en", "service@revolut.com", "Revolut", "Your monthly statement",
  "Your statement for May is ready in the app. You spent €1,242 across 48 transactions.")

out = Path(__file__).resolve().parent / "triage_written.jsonl"
with out.open("w", encoding="utf-8", newline="\n") as f:
    for r in rows:
        f.write(json.dumps(r, ensure_ascii=False) + "\n")
from collections import Counter
print(len(rows), Counter((r["lang"], r["label"]) for r in rows))
