# ExamYO

Aplicație de antrenament pentru examenul de radioamator YO. Răspunzi întâi, abia apoi vezi dacă ai nimerit și, doar dacă vrei, rezolvarea.

Pentru Windows e **un singur `ExamYO.exe` portabil**: nu se instalează, nu are nevoie de Java sau de internet, iar întrebările și figurile sunt incluse în el.

## Ce face

- **Utilizatori locali.** Fiecare își alege numele (și indicativul) la pornire și are progresul lui. Util și la radioclub, pe un calculator comun.
- **Alegerea clasei.** Clasele I și II includ și întrebările claselor inferioare (I = I + II + III, II = II + III). Le iei la rând, de la nivelul cel mai ușor, iar dublurile dintre baze sunt eliminate automat.
- **Parcurgere la rând, fără limită de timp.** Aplicația ține minte unde ai rămas la fiecare probă. Ordinea variantelor de răspuns se amestecă la fiecare afișare, ca să nu memorezi litera.
- **Doar greșelile.** Reiei întrebările la care ultimul răspuns a fost greșit.
- **Statistici.** Pe probe: câte ai parcurs, câte ai nimerit din prima, câte sunt greșite acum, timpul de studiu și proba la care stai cel mai slab. Plus un tabel cu toți utilizatorii.
- **Fără calculator sau ciornă în aplicație**, intenționat: la ANCOM folosești ce ai tu.

Progresul se salvează în `ExamYO-date.json`, lângă `.exe`, deci se mută odată cu aplicația (inclusiv pe stick). Nimic nu pleacă de pe calculator.

## Descărcare

**[Descarcă ultima versiune](https://github.com/Eugen68/examyo-db-app/releases/tag/ultima-versiune)**: `ExamYO-windows.zip`, se dezarhivează și se pornește `ExamYO.exe`.

Fiecare push pe `main` reconstruiește exe-ul în GitHub Actions și actualizează automat acest Release.

## Date

Întrebările vin din [examyo-db](https://github.com/Eugen68/examyo-db) (submodulul `examyo-db/`), care le convertește din [Sim_exam_yo](https://github.com/6oskarwN/Sim_exam_yo) al lui **Francisc Toth, YO6OWN**, autorul site-ului [examyo.scienceontheweb.net](http://examyo.scienceontheweb.net). Tot meritul pentru întrebări și rezolvări le aparține lui și contributorilor menționați la fiecare întrebare.

Greșelile din întrebări se raportează la sursă, pe site, la *Colectare erori și sugestii*.

## Compilare

Ai nevoie de [Rust](https://rustup.rs).

```sh
git clone --recurse-submodules https://github.com/Eugen68/examyo-db-app
cd examyo-db-app
cargo run --release
```

Pentru a lua ultima versiune a întrebărilor: `git submodule update --remote examyo-db`, apoi recompilezi.

Structura:

- `src/data.rs`: încărcarea întrebărilor, compunerea probelor pe clase, eliminarea dublurilor.
- `src/store.rs`: profiluri, progres, statistici și salvarea în JSON.
- `src/app.rs`: interfața (egui).

`data` și `store` nu depind de interfață, ca să poată fi refolosite la o variantă de Android.

## Licență

GNU GPL v2 sau ulterioară, ca datele lui YO6OWN pe care le include (vezi `LICENSE`). Fontul DejaVu Sans are licența sa liberă, în `assets/fonts/LICENSE-DejaVu.txt`.
