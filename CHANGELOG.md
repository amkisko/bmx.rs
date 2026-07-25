# CHANGELOG

## Unreleased

## 0.1.4 (2026-07-25)

- Shrink the release binary with thin LTO, symbol stripping, and a single codegen unit.

## 0.1.3 (2026-03-30)

- Added runtime isolation in addition to build isolation: apps can now run through `docker` / `podman` / `nerdctl` backends.
- New persistent config key `run_isolation` with CLI support: `bmx isolation set-default-run MODE` and `bmx isolation show-run`.
- New per-invocation runtime overrides: `--isolate-run` (force isolated run with auto backend resolution) and `--no-isolate-run` (force host run).
- `bmx doctor` now reports both build and run isolation defaults.

- Contain  / executable paths under the install tree (reject absolute paths and ).
- Sync OpenSSH public keys from trust policy into per-source ; document fingerprint-only limitation.
- Typed process exit codes (usage/trust/network/io) with  hints;  and .
- HTTP fetches use a shared ureq agent with timeout, redirect cap, https-only, and body size limit.
- Defaults: , ; consent no longer skips on local good signature alone; optional .
- Release tooling:  (, ), Makefile, CI gates; packaging installs completions/man.
- Global  for non-interactive runs; trust auto-consent still needs .
-  confirms on a TTY; use  in scripts or with .
-  on , , , and pkcs11:id=%3A%9A%85%07%10%67%28%B6%EF%F6%BD%05%41%6E%20%C1%94%DA%0F%DE;type=cert
    type: certificate
    label: Go Daddy Root Certificate Authority - G2
    trust: anchor
    category: authority

pkcs11:id=%C9%1B%53%81%12%FE%04%D5%16%D1%AA%BC%9A%6F%B7%A0%95%19%6E%CA;type=cert
    type: certificate
    label: HARICA TLS ECC Root CA 2021
    trust: anchor
    category: authority

pkcs11:id=%D2%9F%88%DF%A1%CD%2C%BD%EC%F5%3B%01%01%93%33%27%B2%EB%60%4B;type=cert
    type: certificate
    label: NAVER Global Root Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%C1%51%45%50%59%AB%3E%E7%2C%5A%FA%20%22%12%07%80%88%7C%11%6A;type=cert
    type: certificate
    label: DigiCert TLS ECC P384 Root G5
    trust: anchor
    category: authority

pkcs11:id=%F6%77%6A%DD%3F%FD%01%13%FF%16%A1%6C%82%02%2F%D0%0A%3D%14%25;type=cert
    type: certificate
    label: Sectigo Public Time Stamping Root R46
    trust: anchor
    category: authority

pkcs11:id=%B3%03%7E%AE%36%BC%B0%79%D1%DC%94%26%B6%11%BE%21%B2%69%86%94;type=cert
    type: certificate
    label: OISTE WISeKey Global Root GA CA
    trust: anchor
    category: authority

pkcs11:id=%55%A9%84%89%D2%C1%32%BD%18%CB%6C%A6%07%4E%C8%E7%9D%BE%82%90;type=cert
    type: certificate
    label: Trustwave Global ECC P384 Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%52%D8%88%3A%C8%9F%78%66%ED%89%F3%7B%38%70%94%C9%02%02%36%D0;type=cert
    type: certificate
    label: Actalis Authentication Root CA
    trust: anchor
    category: authority

pkcs11:id=%3F%90%C8%7D%C7%15%6F%F3%24%8F%A9%C3%2F%4B%A2%0F%21%B2%2F%E7;type=cert
    type: certificate
    label: D-TRUST Root CA 3 2013
    trust: anchor
    category: authority

pkcs11:id=%C4%99%13%6C%18%03%C2%7B%C0%A3%A0%0D%7F%72%80%7A%1C%77%26%8D;type=cert
    type: certificate
    label: Apple Root CA - G2
    trust: anchor
    category: authority

pkcs11:id=%5B%CA%5E%E5%DE%D2%81%AA%CD%A8%2D%64%51%B6%D9%72%9B%97%E6%4F;type=cert
    type: certificate
    label: SSL.com EV Root Certification Authority ECC
    trust: anchor
    category: authority

pkcs11:id=%54%B0%7B%AD%45%B8%E2%40%7F%FB%0A%6E%FB%BE%33%C9%3C%A3%84%D5;type=cert
    type: certificate
    label: GlobalSign
    trust: anchor
    category: authority

pkcs11:id=%D1%A3%D4%57%1D%4F%55%DB%75%4C%5C%42%9E%63%16%CE%B4%C6%3B%1F;type=cert
    type: certificate
    label: DigiCert SMIME RSA4096 Root G5
    trust: anchor
    category: authority

pkcs11:id=%71%15%67%C8%C8%C9%BD%75%5D%72%D0%38%18%6A%9D%F3%71%24%54%0B;type=cert
    type: certificate
    label: Hellenic Academic and Research Institutions RootCA 2015
    trust: anchor
    category: authority

pkcs11:id=%1E%0C%F7%B6%67%F2%E1%92%26%09%45%C0%55%39%2E%77%3F%42%4A%A2;type=cert
    type: certificate
    label: ePKI Root Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%A0%11%0A%23%3E%96%F1%07%EC%E2%AF%29%EF%82%A5%7F%D0%30%A4%B4;type=cert
    type: certificate
    label: AAA Certificate Services
    trust: anchor
    category: authority

pkcs11:id=%7F%D3%65%A7%C2%DD%EC%BB%F0%30%09%F3%43%39%FA%02%AF%33%31%33;type=cert
    type: certificate
    label: VeriSign Class 3 Public Primary Certification Authority - G5
    trust: anchor
    category: authority

pkcs11:id=%FB%2E%37%EE%E3%84%7A%27%2E%CD%19%35%B1%33%7C%FF%D4%44%42%B9;type=cert
    type: certificate
    label: SSL.com TLS RSA Root CA 2022
    trust: anchor
    category: authority

pkcs11:id=%BB%B0%DE%A1%58%33%88%9A%A4%8A%99%DE%BE%BD%EB%AF%DA%CB%24%AB;type=cert
    type: certificate
    label: Apple Root CA - G3
    trust: anchor
    category: authority

pkcs11:id=%E3%71%E0%9E%D8%A7%42%D9%DB%71%91%6B%94%93%EB%C3%A3%D1%14%A3;type=cert
    type: certificate
    label: IdenTrust Public Sector Root CA 1
    trust: anchor
    category: authority

pkcs11:id=%C6%17%D0%BC%A8%EA%02%43%F2%1B%06%99%5D%2B%90%20%B9%D7%9C%E4;type=cert
    type: certificate
    label: QuoVadis Root CA 3 G3
    trust: anchor
    category: authority

pkcs11:id=%57%17%ED%A2%CF%DC%7C%98%A1%10%E0%FC%BE%87%2D%2C%F2%E3%17%54;type=cert
    type: certificate
    label: Developer ID Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%D2%87%B4%E3%DF%37%27%93%55%F6%56%EA%81%E5%36%CC%8C%1E%3F%BD;type=cert
    type: certificate
    label: ACCVRAIZ1
    trust: anchor
    category: authority

pkcs11:id=%47%B8%CD%FF%E5%6F%EE%F8%B2%EC%2F%4E%0E%F9%25%B0%8E%3C%6B%C3;type=cert
    type: certificate
    label: Buypass Class 3 Root CA
    trust: anchor
    category: authority

pkcs11:id=%9C%5F%00%DF%AA%01%D7%30%2B%38%88%A2%B8%6D%4A%9C%F2%11%91%83;type=cert
    type: certificate
    label: Starfield Services Root Certificate Authority - G2
    trust: anchor
    category: authority

pkcs11:id=%74%49%99%D1%FF%B4%7A%68%45%75%C3%7E%B4%DC%CC%CE%39%33%DA%08;type=cert
    type: certificate
    label: Atos TrustedRoot Root CA RSA TLS 2021
    trust: anchor
    category: authority

pkcs11:id=%89%8F%2F%A3%E8%2B%A0%14%54%7B%F3%56%B8%26%5F%67%38%0B%9C%D0;type=cert
    type: certificate
    label: SSL.com TLS ECC Root CA 2022
    trust: anchor
    category: authority

pkcs11:id=%44%9E%48%F5%CC%6D%48%D4%A0%4B%7F%FE%59%24%2F%83%97%99%9A%86;type=cert
    type: certificate
    label: TrustCor ECA-1
    trust: anchor
    category: authority

pkcs11:id=%9D%93%C6%53%8B%5E%CA%AF%3F%9F%1E%0F%E5%99%95%BC%24%F6%94%8F;type=cert
    type: certificate
    label: AffirmTrust Commercial
    trust: anchor
    category: authority

pkcs11:id=%31%0A%90%8F%B6%C6%9D%D2%44%4B%80%B5%A2%E6%1F%B1%12%4F%1B%95;type=cert
    type: certificate
    label: GlobalSign Root E46
    trust: anchor
    category: authority

pkcs11:id=%0A%48%23%A6%60%A4%92%0A%33%EA%93%5B%C5%57%EA%25%4D%BD%12%EE;type=cert
    type: certificate
    label: HARICA TLS RSA Root CA 2021
    trust: anchor
    category: authority

pkcs11:id=%17%9D%CD%1E%8B%D6%39%2B%70%D3%5C%D4%A0%B8%1F%B0%00%FC%C5%61;type=cert
    type: certificate
    label: Hongkong Post Root CA 3
    trust: anchor
    category: authority

pkcs11:id=%D1%22%DA%4C%59%F1%4B%5F%26%38%AA%9D%D6%EE%EB%0D%C3%FB%A9%61;type=cert
    type: certificate
    label: Sectigo Public Server Authentication Root E46
    trust: anchor
    category: authority

pkcs11:id=%E0%AA%3F%25%8D%9F%44%5C%C1%3A%E8%2E%AE%77%4C%84%3E%67%0C%F4;type=cert
    type: certificate
    label: Certainly Root R1
    trust: anchor
    category: authority

pkcs11:id=%A3%97%D6%F3%5E%A2%10%E1%AB%45%9F%3C%17%64%3C%EE%01%70%9C%CC;type=cert
    type: certificate
    label: QuoVadis Root CA 1 G3
    trust: anchor
    category: authority

pkcs11:id=%DF%13%5E%8B%5F%C2%40%02%FD%56%B7%94%4C%B6%1E%D5%A6%B1%14%96;type=cert
    type: certificate
    label: GlobalSign Secure Mail Root E45
    trust: anchor
    category: authority

pkcs11:id=%5B%1F%C4%71%6C%B2%1B%9F%BE%5C%1F%8C%FD%B3%B6%FB%B3%0E%09%87;type=cert
    type: certificate
    label: Atos TrustedRoot Root CA ECC G2 2020
    trust: anchor
    category: authority

pkcs11:id=%21%30%C9%FB%00%D7%4E%98%DA%87%AA%2A%D0%A7%2E%B1%40%31%A7%4C;type=cert
    type: certificate
    label: Network Solutions Certificate Authority
    trust: anchor
    category: authority

pkcs11:id=%79%B4%59%E6%7B%B6%E5%E4%01%73%80%08%88%C8%1A%58%F6%E9%9B%6E;type=cert
    type: certificate
    label: ISRG Root X1
    trust: anchor
    category: authority

pkcs11:id=%C6%4F%A2%3D%06%63%84%09%9C%CE%62%E4%04%AC%8D%5C%B5%E9%B6%1B;type=cert
    type: certificate
    label: XRamp Global Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%D9%FE%21%40%6E%94%9E%BC%9B%3D%9C%7D%98%20%19%E5%8C%30%62%B2;type=cert
    type: certificate
    label: TrustCor RootCert CA-2
    trust: anchor
    category: authority

pkcs11:id=%8D%06%66%74%24%76%3A%F3%89%F7%BC%D6%BD%47%7D%2F%BC%10%5F%4B;type=cert
    type: certificate
    label: Certum EC-384 CA
    trust: anchor
    category: authority

pkcs11:id=%F0%38%42%94%34%A9%3C%00%7F%52%EE%39%A5%F7%4B%0D%BC%6A%7D%23;type=cert
    type: certificate
    label: SSL.com Client RSA Root CA 2022
    trust: anchor
    category: authority

pkcs11:id=%D3%94%8A%4C%62%13%2A%19%2E%CC%AF%72%8A%7D%36%D7%9A%1C%DC%67;type=cert
    type: certificate
    label: D-TRUST Root Class 3 CA 2 EV 2009
    trust: anchor
    category: authority

pkcs11:id=%A7%D7%95%77%EB%4A%C3%27%CD%93%BE%37%4C%26%84%21%14%7D%5D%98;type=cert
    type: certificate
    label: Sectigo Public Email Protection Root R46
    trust: anchor
    category: authority

pkcs11:id=%35%0F%C8%36%63%5E%E2%A3%EC%F9%3B%66%15%CE%51%52%E3%91%9A%3D;type=cert
    type: certificate
    label: OISTE WISeKey Global Root GB CA
    trust: anchor
    category: authority

pkcs11:id=%02%45%93%D8%0D%48%62%AC%69%BA%AE%06%5B%3E%FB%AA%26%91%50%B1;type=cert
    type: certificate
    label: ComSign Global Root CA
    trust: anchor
    category: authority

pkcs11:id=%1D%1C%65%0E%A8%F2%25%7B%B4%91%CF%E4%B1%B1%E6%BD%55%74%6C%05;type=cert
    type: certificate
    label: Izenpe.com
    trust: anchor
    category: authority

pkcs11:id=%0B%58%E5%8B%C6%4C%15%37%A4%40%A9%30%A9%21%BE%47%36%5A%56%FF;type=cert
    type: certificate
    label: COMODO Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%EE%6B%49%3C%7A%3F%0D%E3%B1%09%B7%8A%C8%AB%19%9F%73%33%50%E7;type=cert
    type: certificate
    label: TrustCor RootCert CA-1
    trust: anchor
    category: authority

pkcs11:id=%3D%E6%29%48%9B%EA%07%CA%21%44%4A%26%DE%6E%DE%D2%83%D0%9F%59;type=cert
    type: certificate
    label: GlobalSign
    trust: anchor
    category: authority

pkcs11:id=%F7%7D%C5%FD%C4%E8%9A%1B%77%64%A7%F5%1D%A0%CC%BF%87%60%9A%6D;type=cert
    type: certificate
    label: AC RAIZ FNMT-RCM
    trust: anchor
    category: authority

pkcs11:id=%EC%D7%E3%82%D2%71%5D%64%4C%DF%2E%67%3F%E7%BA%98%AE%1C%0F%4F;type=cert
    type: certificate
    label: DigiCert Trusted Root G4
    trust: anchor
    category: authority

pkcs11:id=%6A%38%5B%26%8D%DE%8B%5A%F2%4F%7A%54%83%19%18%E3%08%35%A6%BA;type=cert
    type: certificate
    label: TWCA Root Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%BF%5F%B7%D1%CE%DD%1F%86%F4%5B%55%AC%DC%D7%10%C2%0E%A9%88%E7;type=cert
    type: certificate
    label: Starfield Class 2 Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%A0%D6%07%3D%5E%24%F7%7B%A0%44%2E%24%52%0D%19%AA%2B%04%91%A7;type=cert
    type: certificate
    label: HARICA Client RSA Root CA 2021
    trust: anchor
    category: authority

pkcs11:id=%42%32%B6%16%FA%04%FD%FE%5D%4B%7A%C3%FD%F7%4C%40%1D%5A%43%AF;type=cert
    type: certificate
    label: SecureTrust CA
    trust: anchor
    category: authority

pkcs11:id=%6A%72%26%7A%D0%1E%EF%7D%E7%3B%69%51%D4%6C%8D%9F%90%12%66%AB;type=cert
    type: certificate
    label: Entrust Root Certification Authority - G2
    trust: anchor
    category: authority

pkcs11:id=%51%33%1C%ED%36%40%AF%17%D3%25%CD%69%68%F2%AF%4E%23%3E%B3%41;type=cert
    type: certificate
    label: DigiCert TLS RSA4096 Root G5
    trust: anchor
    category: authority

pkcs11:id=%52%08%D2%BE%32%81%25%FD%F5%1A%97%EC%4E%5F%1A%BB%53%CD%90%AD;type=cert
    type: certificate
    label: HARICA Client ECC Root CA 2021
    trust: anchor
    category: authority

pkcs11:id=%B5%99%F8%AF%B0%94%F5%E3%20%D6%0A%AD%CE%4E%56%A4%2E%6E%42%ED;type=cert
    type: certificate
    label: CA Disig Root R2
    trust: anchor
    category: authority

pkcs11:id=%60%7B%66%1A%45%0D%97%CA%89%50%2F%7D%04%CD%34%A8%FF%FC%FD%4B;type=cert
    type: certificate
    label: GlobalSign Root CA
    trust: anchor
    category: authority

pkcs11:id=%ED%E7%6F%76%5A%BF%60%EC%49%5B%C6%A5%77%BB%72%16%71%9B%C4%3D;type=cert
    type: certificate
    label: QuoVadis Root CA 2 G3
    trust: anchor
    category: authority

pkcs11:id=%45%EB%A2%AF%F4%92%CB%82%31%2D%51%8B%A7%A7%21%9D%F3%6D%C8%0F;type=cert
    type: certificate
    label: DigiCert Assured ID Root CA
    trust: anchor
    category: authority

pkcs11:id=%CB%D0%BD%A9%E1%98%05%51%A1%4D%37%A2%83%79%CE%8D%1D%2A%E4%84;type=cert
    type: certificate
    label: DigiCert Assured ID Root G3
    trust: anchor
    category: authority

pkcs11:id=%99%E0%19%67%0D%62%DB%76%B3%DA%3D%B8%5B%E8%FD%42%D2%31%0E%87;type=cert
    type: certificate
    label: Trustwave Global Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%B7%FE%2D%62%C5%81%53%CD%52%1A%2F%5D%60%A0%C3%5D%FB%B2%1C%1C;type=cert
    type: certificate
    label: SSL.com Client ECC Root CA 2022
    trust: anchor
    category: authority

pkcs11:id=%E2%C9%40%9F%4D%CE%E8%9A%A1%7C%CF%0E%3F%65%C5%29%88%6A%19%51;type=cert
    type: certificate
    label: GDCA TrustAUTH R5 ROOT
    trust: anchor
    category: authority

pkcs11:id=%BF%59%20%36%00%79%A0%A0%22%6B%8C%D5%F2%61%D2%B8%2C%CB%82%4A;type=cert
    type: certificate
    label: T-TeleSec GlobalRoot Class 2
    trust: anchor
    category: authority

pkcs11:id=%B5%03%F7%76%3B%61%82%6A%12%AA%18%53%EB%03%21%94%BF%FE%CE%CA;type=cert
    type: certificate
    label: T-TeleSec GlobalRoot Class 3
    trust: anchor
    category: authority

pkcs11:id=%CE%C3%4A%B9%99%55%F2%B8%DB%60%BF%A9%7E%BD%56%B5%97%36%A7%D6;type=cert
    type: certificate
    label: DigiCert Assured ID Root G2
    trust: anchor
    category: authority

pkcs11:id=%0A%85%A9%77%65%05%98%7C%40%81%F8%0F%97%2C%38%F1%0A%EC%3C%CF;type=cert
    type: certificate
    label: Security Communication RootCA2
    trust: anchor
    category: authority

pkcs11:id=%55%E4%81%D1%11%80%BE%D8%89%B9%08%A3%31%F9%A1%24%09%16%B9%70;type=cert
    type: certificate
    label: Entrust.net Certification Authority (2048)
    trust: anchor
    category: authority

pkcs11:id=%07%1F%D2%E7%9C%DA%C2%6E%A2%40%B4%B0%7A%50%10%50%74%C4%C8%BD;type=cert
    type: certificate
    label: AffirmTrust Networking
    trust: anchor
    category: authority

pkcs11:id=%65%CD%EB%AB%35%1E%00%3E%7E%D5%74%C0%1C%B4%73%47%0E%1A%64%2F;type=cert
    type: certificate
    label: Autoridad de Certificacion Firmaprofesional CIF A62634068
    trust: anchor
    category: authority

pkcs11:id=%AB%B6%DB%D7%06%9E%37%AC%30%86%07%91%70%C7%9C%C4%19%B1%78%C0;type=cert
    type: certificate
    label: Amazon Root CA 3
    trust: anchor
    category: authority

pkcs11:id=%73%7A%6B%96%DB%42%07%8B%52%66%C2%64%32%17%FE%E0%67%90%2E%AD;type=cert
    type: certificate
    label: DigiCert SMIME ECC P384 Root G5
    trust: anchor
    category: authority

pkcs11:id=%B0%0C%F0%4C%30%F4%05%58%02%48%FD%33%E5%52%AF%4B%84%E3%66%52;type=cert
    type: certificate
    label: Amazon Root CA 2
    trust: anchor
    category: authority

pkcs11:id=%76%28%25%D6%7D%E0%66%9A%7A%09%B2%6A%3B%8E%33%D7%36%D3%4F%A2;type=cert
    type: certificate
    label: Atos TrustedRoot Root CA ECC TLS 2021
    trust: anchor
    category: authority

pkcs11:id=%9A%AF%29%7A%C0%11%35%35%26%51%30%00%C3%6A%FE%40%D5%AE%D6%3C;type=cert
    type: certificate
    label: AffirmTrust Premium ECC
    trust: anchor
    category: authority

pkcs11:id=%5B%25%7B%96%A4%65%51%7E%B8%39%F3%C0%78%66%5E%E8%3A%E7%F0%EE;type=cert
    type: certificate
    label: SwissSign Gold CA - G2
    trust: anchor
    category: authority

pkcs11:id=%D2%C4%B0%D2%91%D4%4C%11%71%B3%61%CB%3D%A1%FE%DD%A8%6A%D4%E3;type=cert
    type: certificate
    label: Go Daddy Class 2 Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%82%D1%85%73%30%E7%35%04%D3%8E%02%92%FB%E5%A4%D1%C4%21%E8%CD;type=cert
    type: certificate
    label: SSL.com Root Certification Authority ECC
    trust: anchor
    category: authority

pkcs11:id=%E3%FE%2D%FD%28%D0%0B%B5%BA%B6%A2%C4%BF%06%AA%05%8C%93%FB%2F;type=cert
    type: certificate
    label: CFCA EV ROOT
    trust: anchor
    category: authority

pkcs11:id=%82%21%2D%66%C6%D7%A0%E0%15%EB%CE%4C%09%77%C4%60%9E%54%6E%03;type=cert
    type: certificate
    label: certSIGN ROOT CA G2
    trust: anchor
    category: authority

pkcs11:id=%48%87%14%AC%E3%C3%9E%90%60%3A%D7%CA%89%EE%D3%AD%8C%B4%50%66;type=cert
    type: certificate
    label: OISTE WISeKey Global Root GC CA
    trust: anchor
    category: authority

pkcs11:id=%84%18%CC%85%34%EC%BC%0C%94%94%2E%08%59%9C%C7%B2%10%4E%0A%08;type=cert
    type: certificate
    label: Amazon Root CA 1
    trust: anchor
    category: authority

pkcs11:id=%C9%80%77%E0%62%92%82%F5%46%9C%F3%BA%F7%4C%C3%DE%B8%A3%AD%39;type=cert
    type: certificate
    label: Buypass Class 2 Root CA
    trust: anchor
    category: authority

pkcs11:id=%7C%42%96%AE%DE%4B%48%3B%FA%92%F8%9E%8C%CF%6D%8B%A9%72%37%95;type=cert
    type: certificate
    label: ISRG Root X2
    trust: anchor
    category: authority

pkcs11:id=%FB%EF%0D%86%9E%B0%E3%DD%A9%B9%F1%21%17%7F%3E%FC%F0%77%2B%1A;type=cert
    type: certificate
    label: emSign Root CA - G1
    trust: anchor
    category: authority

pkcs11:id=%15%5F%35%57%51%55%FB%25%B2%AD%03%69%FC%01%A3%FA%BE%11%55%D5;type=cert
    type: certificate
    label: GeoTrust Primary Certification Authority - G2
    trust: anchor
    category: authority

pkcs11:id=%28%A4%BA%EE%61%3E%0A%B8%15%83%95%65%4E%4F%CC%13%C1%70%E3%E3;type=cert
    type: certificate
    label: TWCA Global Root CA
    trust: anchor
    category: authority

pkcs11:id=%AF%44%04%C2%41%7E%48%83%DB%4E%39%02%EC%EC%84%7A%E6%CE%C9%A4;type=cert
    type: certificate
    label: Secure Global CA
    trust: anchor
    category: authority

pkcs11:id=%D3%EC%C7%3A%65%6E%CC%E1%DA%76%9A%56%FB%9C%F3%86%6D%57%E5%81;type=cert
    type: certificate
    label: Amazon Root CA 4
    trust: anchor
    category: authority

pkcs11:id=%9D%C0%67%A6%0C%22%D9%26%F5%45%AB%A6%65%52%11%27%D8%45%AC%63;type=cert
    type: certificate
    label: AffirmTrust Premium
    trust: anchor
    category: authority

pkcs11:id=%17%A0%CD%C1%E4%41%B6%3A%5B%3B%CB%45%9D%BD%1C%C2%98%FA%86%58;type=cert
    type: certificate
    label: SwissSign Silver CA - G2
    trust: anchor
    category: authority

pkcs11:id=%20%25%F3%07%FD%A7%6F%F1%96%EE%91%10%69%CC%9A%EF%7D%C8%68%78;type=cert
    type: certificate
    label: Atos TrustedRoot Root CA RSA G2 2020
    trust: anchor
    category: authority

pkcs11:id=%CC%FA%67%93%F0%B6%B8%D0%A5%C0%1E%F3%53%FD%8C%53%DF%83%D7%96;type=cert
    type: certificate
    label: NetLock Arany (Class Gold) Főtanúsítvány
    trust: anchor
    category: authority

pkcs11:id=%7C%5D%02%84%13%D4%CC%8A%9B%81%CE%17%1C%2E%29%1E%9C%48%63%42;type=cert
    type: certificate
    label: emSign ECC Root CA - G3
    trust: anchor
    category: authority

pkcs11:id=%2D%4E%8C%A7%C2%23%B2%57%A9%06%6B%3E%6B%2B%89%F3%C3%5E%47%CE;type=cert
    type: certificate
    label: Sectigo Public Email Protection Root E46
    trust: anchor
    category: authority

pkcs11:id=%F2%77%17%FA%5E%A8%FE%F6%3D%71%D5%68%BA%C9%46%0C%38%D8%AF%B0;type=cert
    type: certificate
    label: HiPKI Root CA - G1
    trust: anchor
    category: authority

pkcs11:id=%2B%D0%69%47%94%76%09%FE%F4%6B%8D%2E%40%A6%F7%47%4D%7F%08%5E;type=cert
    type: certificate
    label: Apple Root CA
    trust: anchor
    category: authority

pkcs11:id=%F9%24%AC%0F%B2%B5%F8%79%C0%FA%60%88%1B%C4%D9%4D%02%9E%17%19;type=cert
    type: certificate
    label: Chambers of Commerce Root - 2008
    trust: anchor
    category: authority

pkcs11:id=%8C%FB%1C%75%BC%02%D3%9F%4E%2E%48%D9%F9%60%54%AA%C4%B3%4F%FA;type=cert
    type: certificate
    label: Certum Trusted Root CA
    trust: anchor
    category: authority

pkcs11:id=%75%71%A7%19%48%19%BC%9D%9D%EA%41%47%DF%94%C4%48%77%99%D3%79;type=cert
    type: certificate
    label: COMODO ECC Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%A0%93%15%28%6E%EE%8F%08%B2%35%C6%9E%62%79%74%A7%B1%0E%2B%7B;type=cert
    type: certificate
    label: GlobalSign Secure Mail Root R45
    trust: anchor
    category: authority

pkcs11:id=%E4%AF%2B%26%71%1A%2B%48%27%85%2F%52%66%2C%EF%F0%89%13%71%3E;type=cert
    type: certificate
    label: GTS Root R1
    trust: anchor
    category: authority

pkcs11:id=%65%3F%C7%8A%86%C6%3C%DD%3C%54%5C%35%F8%3A%ED%52%0C%47%57%C8;type=cert
    type: certificate
    label: TUBITAK Kamu SM SSL Kok Sertifikasi - Surum 1
    trust: anchor
    category: authority

pkcs11:id=%A3%41%06%AC%90%6D%D1%4A%EB%75%A5%4A%10%99%B3%B1%A1%8B%4A%F7;type=cert
    type: certificate
    label: Trustwave Global ECC P256 Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%B7%63%E7%1A%DD%8D%E9%08%A6%55%83%A4%E0%6A%50%41%65%11%42%49;type=cert
    type: certificate
    label: Entrust Root Certification Authority - EC1
    trust: anchor
    category: authority

pkcs11:id=%68%90%E4%67%A4%A6%53%80%C7%86%66%A4%F1%F7%4B%43%FB%84%BD%6D;type=cert
    type: certificate
    label: Entrust Root Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%A7%A5%06%B1%2C%A6%09%60%EE%D1%97%E9%70%AE%BC%3B%19%6C%DB%21;type=cert
    type: certificate
    label: Atos TrustedRoot 2011
    trust: anchor
    category: authority

pkcs11:id=%FD%DA%14%C4%9F%30%DE%21%BD%1E%42%39%FC%AB%63%23%49%E0%F1%84;type=cert
    type: certificate
    label: D-TRUST Root Class 3 CA 2 2009
    trust: anchor
    category: authority

pkcs11:id=%B1%3E%C3%69%03%F8%BF%47%01%D4%98%26%1A%08%02%EF%63%64%2B%C3;type=cert
    type: certificate
    label: DigiCert High Assurance EV Root CA
    trust: anchor
    category: authority

pkcs11:id=%08%76%CD%CB%07%FF%24%F6%C5%CD%ED%BB%90%BC%E2%84%37%46%75%F7;type=cert
    type: certificate
    label: Certum Trusted Network CA
    trust: anchor
    category: authority

pkcs11:id=%56%73%58%64%95%F9%92%1A%B0%12%2A%04%62%79%A1%40%15%88%21%49;type=cert
    type: certificate
    label: Sectigo Public Server Authentication Root R46
    trust: anchor
    category: authority

pkcs11:id=%03%5C%AB%73%81%87%A8%CC%B0%A6%D5%94%E2%36%96%49%FF%05%99%2C;type=cert
    type: certificate
    label: GlobalSign Root R46
    trust: anchor
    category: authority

pkcs11:id=%B9%09%CA%9C%1E%DB%D3%6C%3A%6B%AE%ED%54%F1%5B%93%06%35%2E%5E;type=cert
    type: certificate
    label: Global Chambersign Root - 2008
    trust: anchor
    category: authority

pkcs11:id=%72%AC%E4%33%79%AA%45%87%F6%FD%AC%1D%9E%D6%C7%2F%86%D8%24%39;type=cert
    type: certificate
    label: Telia Root CA v2
    trust: anchor
    category: authority

pkcs11:id=%ED%44%19%C0%D3%F0%06%8B%EE%A4%7B%BE%42%E7%26%54%C8%8E%36%76;type=cert
    type: certificate
    label: IdenTrust Commercial Root CA 1
    trust: anchor
    category: authority

pkcs11:id=%BB%FF%CA%8E%23%9F%4F%99%CA%DB%E2%68%A6%A5%15%27%17%1E%D9%0E;type=cert
    type: certificate
    label: GTS Root R2
    trust: anchor
    category: authority

pkcs11:id=%8F%F0%4B%7F%A8%2E%45%24%AE%4D%50%FA%63%9A%8B%DE%E2%DD%1B%BC;type=cert
    type: certificate
    label: GlobalSign
    trust: anchor
    category: authority

pkcs11:id=%9F%38%C4%56%23%C3%39%E8%A0%71%6C%E8%54%4C%E4%E8%3A%B1%BF%67;type=cert
    type: certificate
    label: Entrust Root Certification Authority - G4
    trust: anchor
    category: authority

pkcs11:id=%AE%6C%05%A3%93%13%E2%A2%E7%E2%D7%1C%D6%C7%F0%7F%C8%67%53%A0;type=cert
    type: certificate
    label: GlobalSign
    trust: anchor
    category: authority

pkcs11:id=%F0%8F%59%38%00%B3%F5%8F%9A%96%0C%D5%EB%FA%7B%AA%17%E8%13%12;type=cert
    type: certificate
    label: TeliaSonera Root CA v1
    trust: anchor
    category: authority

pkcs11:id=%BB%AF%7E%02%3D%FA%A6%F1%3C%84%8E%AD%EE%38%98%EC%D9%32%32%D4;type=cert
    type: certificate
    label: COMODO RSA Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%C1%F1%26%BA%A0%2D%AE%85%81%CF%D3%F1%2A%12%BD%B8%0A%67%FD%BC;type=cert
    type: certificate
    label: GTS Root R3
    trust: anchor
    category: authority

pkcs11:id=%DD%04%09%07%A2%F5%7A%7D%52%53%12%92%95%EE%38%80%25%0D%A6%59;type=cert
    type: certificate
    label: SSL.com Root Certification Authority RSA
    trust: anchor
    category: authority

pkcs11:id=%27%F3%C8%15%1E%6E%9A%02%09%16%AD%2B%A0%89%60%5F%DA%7B%2F%AA;type=cert
    type: certificate
    label: Cisco Root CA 2048
    trust: anchor
    category: authority

pkcs11:id=%E0%8C%9B%DB%25%49%B3%F1%7C%86%D6%B2%42%87%0B%D0%6B%A0%D9%E4;type=cert
    type: certificate
    label: certSIGN ROOT CA
    trust: anchor
    category: authority

pkcs11:id=%F9%60%BB%D4%E3%D5%34%F6%B8%F5%06%80%25%A7%73%DB%46%69%A8%9E;type=cert
    type: certificate
    label: SSL.com EV Root Certification Authority RSA R2
    trust: anchor
    category: authority

pkcs11:id=%3A%E1%09%86%D4%CF%19%C2%96%76%74%49%76%DC%E0%35%C6%63%63%9A;type=cert
    type: certificate
    label: USERTrust ECC Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%86%1C%E7%FE%2D%A5%4A%8B%08%FE%28%11%FA%BE%A3%66%F8%60%59%2F;type=cert
    type: certificate
    label: Security Communication ECC RootCA1
    trust: anchor
    category: authority

pkcs11:id=%CB%0F%C6%DF%42%43%CC%3D%CB%B5%48%23%A1%1A%7A%A6%2A%BB%34%68;type=cert
    type: certificate
    label: Microsec e-Szigno Root CA 2009
    trust: anchor
    category: authority

pkcs11:id=%C8%CB%99%72%70%52%0C%F8%E6%BE%B2%04%57%29%2A%CF%42%10%ED%35;type=cert
    type: certificate
    label: Microsoft ECC Root Certificate Authority 2017
    trust: anchor
    category: authority

pkcs11:id=%1A%ED%FE%41%39%90%B4%24%59%BE%01%F2%52%D5%45%F6%5A%39%DC%11;type=cert
    type: certificate
    label: Certigna
    trust: anchor
    category: authority

pkcs11:id=%DC%2E%1F%D1%61%37%79%E4%AB%D5%D5%B3%12%71%68%3D%6A%68%9C%22;type=cert
    type: certificate
    label: GLOBALTRUST 2020
    trust: anchor
    category: authority

pkcs11:id=%80%4C%D6%EB%74%FF%49%36%A3%D5%D8%FC%B5%3E%C5%6A%F0%94%1D%8C;type=cert
    type: certificate
    label: GTS Root R4
    trust: anchor
    category: authority

pkcs11:id=%F2%C0%13%E0%82%43%3E%FB%EE%2F%67%32%96%35%5C%DB%B8%CB%02%D0;type=cert
    type: certificate
    label: QuoVadis Root CA 3
    trust: anchor
    category: authority

pkcs11:id=%4E%22%54%20%18%95%E6%E3%6E%E6%0F%FA%FA%B9%12%ED%06%17%8F%39;type=cert
    type: certificate
    label: DigiCert Global Root G2
    trust: anchor
    category: authority

pkcs11:id=%CC%47%3E%AA%15%DD%92%36%25%2F%B0%01%DF%CF%6E%45%C1%5D%DF%2A;type=cert
    type: certificate
    label: Sectigo Public Time Stamping Root E46
    trust: anchor
    category: authority

pkcs11:id=%7C%0C%32%1F%A7%D9%30%7F%C4%7D%68%A3%62%A8%A1%CE%AB%07%5B%27;type=cert
    type: certificate
    label: Starfield Root Certificate Authority - G2
    trust: anchor
    category: authority

pkcs11:id=%B4%22%0B%82%99%24%01%0E%9C%BB%E4%0E%FD%BF%FB%97%20%93%99%2A;type=cert
    type: certificate
    label: Hellenic Academic and Research Institutions ECC RootCA 2015
    trust: anchor
    category: authority

pkcs11:id=%03%DE%50%35%56%D1%4C%BB%66%F0%A3%E2%1B%1B%C3%97%B2%3D%D1%55;type=cert
    type: certificate
    label: DigiCert Global Root CA
    trust: anchor
    category: authority

pkcs11:id=%B3%DB%48%A4%F9%A1%C5%D8%AE%36%41%CC%11%63%69%62%29%BC%4B%C6;type=cert
    type: certificate
    label: DigiCert Global Root G3
    trust: anchor
    category: authority

pkcs11:id=%F3%28%18%CB%64%75%EE%29%2A%EB%ED%AE%23%58%38%85%EB%C8%22%07;type=cert
    type: certificate
    label: Certainly Root E1
    trust: anchor
    category: authority

pkcs11:id=%1A%84%62%BC%48%4C%33%25%04%D4%EE%D0%F6%03%C4%19%46%D1%94%6B;type=cert
    type: certificate
    label: QuoVadis Root CA 2
    trust: anchor
    category: authority

pkcs11:id=%6D%AA%9B%09%87%C4%D0%D4%22%ED%40%07%37%4D%19%F1%91%FF%DE%D3;type=cert
    type: certificate
    label: Certum CA
    trust: anchor
    category: authority

pkcs11:id=%53%79%BF%5A%AA%2B%4A%CF%54%80%E1%D8%9B%C0%9D%F2%B2%03%66%CB;type=cert
    type: certificate
    label: USERTrust RSA Certification Authority
    trust: anchor
    category: authority

pkcs11:id=%09%CB%59%7F%86%B2%70%8F%1A%C3%39%E3%C0%D9%E9%BF%BB%4D%B2%23;type=cert
    type: certificate
    label: Microsoft RSA Root Certificate Authority 2017
    trust: anchor
    category: authority

pkcs11:id=%2E%16%A9%4A%18%B5%CB%CC%F5%6F%50%F3%23%5F%F8%5D%E7%AC%F0%C8;type=cert
    type: certificate
    label: SZAFIR ROOT CA2
    trust: anchor
    category: authority

pkcs11:id=%B6%A1%54%39%02%C3%A0%3F%8E%8A%BC%FA%D4%F8%1C%A6%D1%3A%0E%FD;type=cert
    type: certificate
    label: Certum Trusted Network CA 2
    trust: anchor
    category: authority

pkcs11:id=%81%C4%8C%CC%F5%E4%30%FF%A5%0C%08%5F%8C%15%67%21%74%01%DF%DF;type=cert
    type: certificate
    label: UCA Global G2 Root
    trust: anchor
    category: authority

pkcs11:id=%D9%74%3A%E4%30%3D%0D%F7%12%DC%7E%5A%05%9F%1E%34%9A%F7%E1%14;type=cert
    type: certificate
    label: UCA Extended Validation Root
    trust: anchor
    category: authority

pkcs11:id=%18%87%56%E0%6E%77%EE%24%35%3C%4E%73%9A%1F%D6%E1%E2%79%7E%2B;type=cert
    type: certificate
    label: Certigna Root CA
    trust: anchor
    category: authority

pkcs11:id=%FE%A1%E0%70%1E%2A%03%39%52%5A%42%BE%5C%91%85%7A%18%AA%4D%B5;type=cert
    type: certificate
    label: emSign Root CA - C1
    trust: anchor
    category: authority

pkcs11:id=%FB%5A%48%D0%80%20%40%F2%A8%E9%00%07%69%19%77%A7%E6%C3%F4%CF;type=cert
    type: certificate
    label: emSign ECC Root CA - C3
    trust: anchor
    category: authority

pkcs11:id=%87%11%15%08%D1%AA%C1%78%0C%B1%AF%CE%C6%C9%90%EF%BF%30%04%C0;type=cert
    type: certificate
    label: e-Szigno Root CA 2017
    trust: anchor
    category: authority

pkcs11:id=%01%B9%2F%EF%BF%11%86%60%F2%4F%D0%41%6E%AB%73%1F%E7%D2%6E%49;type=cert
    type: certificate
    label: AC RAIZ FNMT-RCM SERVIDORES SEGUROS
    trust: anchor
    category: authority

pkcs11:id=%9C%5F%D0%6C%63%A3%5F%93%CA%93%98%08%AD%8C%87%A5%2C%5C%C1%37;type=cert
    type: certificate
    label: ANF Secure Server Root CA
    trust: anchor
    category: authority

pkcs11:id=%06%9A%9B%1F%53%7D%F1%F5%A4%C8%D3%86%3E%A1%73%59%B4%F7%44%21;type=cert
    type: certificate
    label: TunTrust Root CA
    trust: anchor
    category: authority

pkcs11:id=%65%CD%EB%AB%35%1E%00%3E%7E%D5%74%C0%1C%B4%73%47%0E%1A%64%2F;type=cert
    type: certificate
    label: Autoridad de Certificacion Firmaprofesional CIF A62634068
    trust: anchor
    category: authority

pkcs11:id=%98%39%CD%BE%D8%B2%8C%F7%B2%AB%E1%AD%24%AF%7B%7C%A1%DB%1F%CF;type=cert
    type: certificate
    label: vTrus ECC Root CA
    trust: anchor
    category: authority

pkcs11:id=%54%62%70%63%F1%75%84%43%58%8E%D1%16%20%B1%C6%AC%1A%BC%F6%89;type=cert
    type: certificate
    label: vTrus Root CA
    trust: anchor
    category: authority

pkcs11:id=%73%91%10%AB%FF%55%B3%5A%7C%09%25%D5%B2%BA%08%A0%6B%AB%1F%6D;type=cert
    type: certificate
    label: D-TRUST BR Root CA 1 2020
    trust: anchor
    category: authority

pkcs11:id=%7F%10%01%16%37%3A%A4%28%E4%50%F8%A4%F7%EC%6B%32%B6%FE%E9%8B;type=cert
    type: certificate
    label: D-TRUST EV Root CA 1 2020
    trust: anchor
    category: authority

pkcs11:id=%C5%EF%ED%CC%D8%8D%21%C6%48%E4%E3%D7%14%2E%A7%16%93%E5%98%01;type=cert
    type: certificate
    label: BJCA Global Root CA1
    trust: anchor
    category: authority

pkcs11:id=%D2%4A%B1%51%7F%06%F0%D1%82%1F%4E%6E%5F%AB%83%FC%48%D4%B0%91;type=cert
    type: certificate
    label: BJCA Global Root CA2
    trust: anchor
    category: authority

pkcs11:id=%40%E4%E4%F2%23%EF%38%CA%B0%AE%57%7F%F2%21%30%16%34%DB%BC%92;type=cert
    type: certificate
    label: TrustAsia Global Root CA G3
    trust: anchor
    category: authority

pkcs11:id=%A5%BB%4A%97%CE%B3%2B%7F%A4%31%DE%97%83%59%83%A6%6F%71%CB%DE;type=cert
    type: certificate
    label: TrustAsia Global Root CA G4
    trust: anchor
    category: authority

pkcs11:id=%E3%72%CC%6E%95%99%47%B1%E6%B3%61%4C%D1%CB%AB%E3%BA%CD%DE%9F;type=cert
    type: certificate
    label: Telekom Security TLS ECC Root 2020
    trust: anchor
    category: authority

pkcs11:id=%B6%A7%97%82%3D%74%85%9B%F7%3C%9F%93%9A%95%79%75%52%8C%6D%47;type=cert
    type: certificate
    label: Telekom Security TLS RSA Root 2023
    trust: anchor
    category: authority

pkcs11:id=%9D%85%61%14%7C%C1%62%6F%97%68%E4%4F%37%40%E1%AD%E0%0D%56%37;type=cert
    type: certificate
    label: TWCA CYBER Root CA
    trust: anchor
    category: authority

pkcs11:id=%57%34%F3%74%CF%04%4B%D5%25%E6%F1%40%B6%2C%4C%D9%2D%E9%A0%AD;type=cert
    type: certificate
    label: SecureSign Root CA12
    trust: anchor
    category: authority

pkcs11:id=%06%93%A3%0A%5E%28%69%37%AA%61%1D%EB%EB%FC%2D%6F%23%E4%F3%A0;type=cert
    type: certificate
    label: SecureSign Root CA14
    trust: anchor
    category: authority

pkcs11:id=%EB%41%C8%AE%FC%D5%9E%51%48%F5%BD%8B%F4%87%20%93%41%2B%D3%F4;type=cert
    type: certificate
    label: SecureSign Root CA15
    trust: anchor
    category: authority

pkcs11:id=%67%90%F0%D6%DE%B5%18%D5%46%29%7E%5C%AB%F8%9E%08%BC%64%95%10;type=cert
    type: certificate
    label: D-TRUST BR Root CA 2 2023
    trust: anchor
    category: authority

pkcs11:id=%2C%85%53%BB%B1%43%CD%32%EA%9E%A3%87%FE%A2%98%A8%A6%93%E9%10;type=cert
    type: certificate
    label: TrustAsia TLS ECC Root CA
    trust: anchor
    category: authority

pkcs11:id=%B8%07%91%79%5C%06%F4%46%FD%7B%59%CA%5A%26%91%A7%45%2B%F8%53;type=cert
    type: certificate
    label: TrustAsia TLS RSA Root CA
    trust: anchor
    category: authority

pkcs11:id=%AA%FC%91%10%1B%87%91%5F%16%B9%BF%4F%4B%91%5E%00%1C%B1%32%80;type=cert
    type: certificate
    label: D-TRUST EV Root CA 2 2023
    trust: anchor
    category: authority

pkcs11:id=%6F%8E%62%8B%93%43%B0%E1%40%F6%A7%C3%FD%F1%0F%B8%0F%15%38%A5;type=cert
    type: certificate
    label: SwissSign RSA TLS Root CA 2022 - 1
    trust: anchor
    category: authority

pkcs11:id=%37%4D%88%65%CF%FC%3D%8A%D5%A3%F1%49%C0%4E%0C%10%6F%42%B4%9C;type=cert
    type: certificate
    label: OISTE Server Root ECC G1
    trust: anchor
    category: authority

pkcs11:id=%F2%C9%C1%0F%0D%63%00%BB%EC%45%0E%4A%1F%B5%B1%B3%36%CD%0E%8D;type=cert
    type: certificate
    label: OISTE Server Root RSA G1
    trust: anchor
    category: authority

pkcs11:id=%59%84%02%62%5A%46%78%F5%5D%DC%8F%0A%10%28%23%DC%D5%D6%FB%45;type=cert
    type: certificate
    label: e-Szigno TLS Root CA 2023
    trust: anchor
    category: authority.
- Bare  prints concise help; ; typo suggestions before treating unknown names as apps.

## 0.1.2 (2026-03-29)

- `bmx trust remove-key` (alias `bmx trust revoke`) removes a signer from the same default or `--match-prefix` scope as `add-key`.
- New `bmx clean [--dry-run]` command removes leftover bmx temp directories (`bmx-ephemeral-*`, `bmx-trust-import-*`, `bmx-trust-check-*`) under the system temp folder.
- New `bmx rebuild [--install] [APP]` command: rebuilds with explicit ref support (`APP@ref`), uses pinned ref when present, and works with `--pin`.
- New trust policy support in `~/.bmx/trust.toml`: source allow/deny rules, optional signed-commit enforcement, and signer allowlists.
- New trust management commands: `bmx trust show|add-key|set-signed|set-allow|import-repo`.
- New global `--trust` consent flow for install/update/rebuild/reinstall/self-update: shows concise HEAD signer details and asks before importing signer keys.
- `--rm` now correctly applies to `bmx shim` commands (no persistent `~/.bmx` writes in ephemeral mode).
- `--rm` seeds ephemeral `BMX_HOME` from the real `~/.bmx/trust.toml` and `~/.bmx/trust/` so trust policy (allowlists, deny rules, SSH allowed signers) still applies; `config.toml` remains isolated.
- Ref pin persistence tightened: refs are persisted only when `--pin` is explicitly used.

## 0.1.1 (2026-03-29)

- `bmx install --as` and `bmx show` (list files under an install; `--would-remove` previews what uninstall would delete); build and install layout handling improvements.
- `bmx history` and `bmx undo` for install/update/reinstall flows, with audit log entries and snapshot-based rollback.
- README: remote bootstrap for `setup.sh` using `curl` and a direct GitHub raw URL.
- Trunk and linter configuration (`.trunk/`); GitHub Actions workflow runs trunk checks on push and pull requests.
- `bmx update <app>` requires an existing install and fails otherwise; use `bmx install` for a first-time install.

## 0.1.0 (2026-03-28)

Initial release.

- CLI to install, build, run, update, reinstall, and uninstall apps from git sources; default command runs after install-if-needed.
- Config and cache under `~/.bmx` (`config.toml`, per-app repo cache and `install.toml`).
- Source resolution: URLs, repo refs, bare names against a configurable default base, and ref-qualified inputs (`app@ref`).
- Build strategy detection (Cargo, CMake, Make, Homebrew-style, AUR) with optional `bmx.toml` workdir and explicit run path.
- Configurable build isolation: off, auto, or a specific OCI backend (`docker`, `podman`, `nerdctl`).
- Subcommands: `exec`, `install`, `uninstall`, `reinstall`, `self-update`, `update`, `source`, `isolation`, `checkout`, `shim`, `doctor`; `source`, `isolation`, and `checkout` each include `show` and `set-default` for defaults.
- `setup.sh` for building from source and installing the `bmx` binary.
- Specification and contributor docs in the repository root.
