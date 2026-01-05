# Rust로 만든 대화형 XML 사이트맵 생성기 (Interactive XML Sitemap Generator)
이 프로젝트는 지정된 웹사이트를 크롤링하여 검색 엔진 최적화(SEO)에 필수적인 sitemap.xml 파일을 생성하는 간단한 Rust 기반의 커맨드 라인 도구입니다. 사용자와의 대화형 인터페이스를 통해 쉽고 빠르게 사이트맵 생성 옵션을 설정할 수 있습니다.

## 주요 기능
- 대화형 CLI: dialoguer 크레이트를 활용하여 사용자에게 친숙한 대화형 프롬프트를 제공합니다.
- 웹사이트 크롤러: reqwest와 scraper를 사용하여 지정된 도메인 내의 모든 페이지를 재귀적으로 탐색하고 수집합니다.
- URL 정규화 (Normalization): 불필요한 URL 프래그먼트(#) 및 추적/세션 파라미터(예: sid, phpsessid)를 제거하여 깨끗하고 표준화된 URL 목록을 생성합니다.
- XML 사이트맵 생성: 수집된 URL 목록을 바탕으로 quick-xml을 사용하여 Sitemap 프로토콜 표준을 준수하는 sitemap.xml 파일을 생성합니다.
- 사용자 정의 옵션: 사이트맵에 포함될 lastmod(마지막 수정일), changefreq(변경 빈도), priority(우선순위) 값을 유연하게 설정할 수 있습니다.

## 사전 준비
이 프로젝트를 빌드하고 실행하려면 시스템에 Rust와 Cargo가 설치되어 있어야 합니다.

## 설치 및 빌드
### 저장소 복제:

```bash
git clone https://github.com/ksrae/rust-sitemap-generator.git
cd rust-sitemap-generator
```

### 프로젝트 빌드 (릴리즈 모드):

```bash
cargo build --release
```

빌드가 완료되면 target/release/ 디렉토리에 실행 파일이 생성됩니다.

## 사용법
빌드된 실행 파일을 직접 실행하거나 cargo run 명령어를 사용하세요.

```bash
# 릴리즈 모드로 실행
./target/release/rust-sitemap-generator

# 또는 Cargo를 통해 실행
cargo run
```

이 프로그램은 커맨드 라인 인자(argument)를 받지 않고, 실행 후 나타나는 대화형 프롬프트에 따라 순서대로 옵션을 설정하는 방식으로 동작합니다.


### 실행 과정
- 사이트맵을 생성할 URL 입력: 크롤링을 시작할 기본 URL을 입력합니다. (예: https://example.com)
- 페이지 변경 빈도 선택: daily, weekly, monthly 등 페이지의 예상 변경 주기를 선택합니다.
- 마지막 수정일 옵션 선택: 모든 URL에 대해 마지막 수정일을 지정하지 않거나, 특정 날짜(YYYY-MM-DD)를 일괄 적용할 수 있습니다.
- 페이지 우선순위 옵션 선택: 모든 URL에 대한 상대적 중요도(0.0 ~ 1.0)를 지정하거나, 지정하지 않을 수 있습니다.
- URL에서 제거할 세션 파라미터 입력: 사이트맵에 포함시키지 않을 URL 쿼리 파라미터를 쉼표로 구분하여 입력합니다. (기본값: sid,phpsessid)
모든 설정이 완료되면 크롤링이 시작되고, 완료 후 현재 디렉토리에 sitemap.xml 파일이 생성됩니다.

## 출력 예시 (sitemap.xml)

```xml
<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://example.com/</loc>
    <lastmod>2023-10-27</lastmod>
    <changefreq>weekly</changefreq>
    <priority>0.8</priority>
  </url>
  <url>
    <loc>https://example.com/about</loc>
    <lastmod>2023-10-27</lastmod>
    <changefreq>weekly</changefreq>
    <priority>0.8</priority>
  </url>
  <url>
    <loc>https://example.com/contact</loc>
    <lastmod>2023-10-27</lastmod>
    <changefreq>weekly</changefreq>
    <priority>0.8</priority>
  </url>
</urlset>
```

## 주요 의존성 (Crates)
- reqwest: HTTP 클라이언트. 웹사이트 페이지를 가져오는 데 사용됩니다.
- scraper: HTML 파싱 및 CSS 셀렉터 엔진. 페이지 내의 링크(<a> 태그)를 추출하는 데 사용됩니다.
- url: URL 파싱 및 조작 라이브러리. URL을 정규화하는 데 사용됩니다.
- quick-xml: 빠르고 효율적인 XML 리더/라이터. sitemap.xml 파일을 생성하는 데 사용됩니다.
- dialoguer: 터미널에서 사용자 입력을 받는 대화형 프롬프트를 만드는 데 사용됩니다.