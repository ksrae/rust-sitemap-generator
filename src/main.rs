use std::collections::{HashSet, VecDeque};
use std::fs::File;
use std::io::BufWriter;

use dialoguer::{theme::ColorfulTheme, Input, Select};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use scraper::{Html, Selector};
use url::{ParseError, Url};
use futures::future::join_all;

// --- 설정 구조체 (String 소유 방식으로 변경) ---
struct SitemapOptions {
    base_url: String,
    output_file: String,
    changefreq: String,
    lastmod_option: LastmodOption,
    priority: Option<f32>,
    session_params_to_remove: Vec<String>,
}

enum LastmodOption {
    None,
    Exact(String),
}
// --- 설정 구조체 끝 ---

// clean_url, crawl_site, generate_xml 함수는 이전 코드와 동일합니다.
// (아래에 전체 코드가 포함되어 있으니 복사해서 사용하시면 됩니다.)

fn clean_url(url: &Url, params_to_remove: &[String]) -> Result<Url, ParseError> {
    let mut cleaned_url = url.clone();
    cleaned_url.set_fragment(None);

    let params_to_remove_str: Vec<&str> = params_to_remove.iter().map(AsRef::as_ref).collect();

    let new_pairs: Vec<(String, String)> = cleaned_url
        .query_pairs()
        .filter(|(key, _)| !params_to_remove_str.contains(&key.as_ref()))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    if new_pairs.is_empty() {
        cleaned_url.set_query(None);
    } else {
        cleaned_url.query_pairs_mut().clear().extend_pairs(new_pairs);
    }

    Ok(cleaned_url)
}

async fn crawl_site(
    start_url: &Url,
    session_params: &[String],
) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    println!("'{}' 사이트 크롤링을 시작합니다...", start_url);

    let base_domain = start_url.domain().ok_or("시작 URL에 도메인이 없습니다.")?.to_string();
    let mut urls_to_visit = VecDeque::new();
    urls_to_visit.push_back(start_url.clone());

    let mut visited = HashSet::new();
    let mut in_flight = HashSet::new();
    let client = reqwest::Client::new();
    let concurrency: usize = 20;

    while !urls_to_visit.is_empty() {
        // Prepare a batch of up to `concurrency` futures
        let mut batch = Vec::new();

        while batch.len() < concurrency {
            if let Some(current_url) = urls_to_visit.pop_front() {
                match clean_url(&current_url, session_params) {
                    Ok(cleaned_url) => {
                        // domain check
                        if let Some(domain) = cleaned_url.domain() {
                            if domain != base_domain { continue; }
                        } else { continue; }

                        let cleaned_str = cleaned_url.to_string();
                        if visited.contains(&cleaned_str) || in_flight.contains(&cleaned_str) { continue; }

                        in_flight.insert(cleaned_str.clone());
                        let client = client.clone();

                        batch.push(tokio::spawn(async move {
                            let mut found_urls: Vec<Url> = Vec::new();

                            let res = match client.get(cleaned_url.clone()).send().await {
                                Ok(response) => {
                                    if !response.status().is_success() {
                                        Err(format!("Status not success: {}", response.status()))
                                    } else {
                                        let content_type = response.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("");
                                        if !content_type.contains("text/html") {
                                            Err("Not an HTML page".to_string())
                                        } else {
                                            match response.text().await {
                                                Ok(body) => {
                                                    println!("  [탐색 완료] {}", cleaned_url);
                                                    let document = Html::parse_document(&body);
                                                    let link_selector = Selector::parse("a[href]").unwrap();
                                                    for element in document.select(&link_selector) {
                                                        if let Some(href) = element.value().attr("href") {
                                                            if let Ok(mut absolute_url) = cleaned_url.join(href) {
                                                                absolute_url.set_fragment(None);
                                                                found_urls.push(absolute_url);
                                                            }
                                                        }
                                                    }
                                                    Ok(found_urls)
                                                }
                                                Err(e) => Err(format!("Body read error: {}", e)),
                                            }
                                        }
                                    }
                                }
                                Err(e) => Err(format!("Request error: {}", e)),
                            };

                            (cleaned_str, res)
                        }));
                    }
                    Err(_) => {
                        // skip invalid URL
                    }
                }
            } else {
                break;
            }
        }

        if batch.is_empty() { break; }

        let results = join_all(batch).await;

        for handle in results {
            if let Ok((cleaned_str, result)) = handle {
                in_flight.remove(&cleaned_str);
                match result {
                    Ok(found_urls) => {
                        visited.insert(cleaned_str.clone());
                        for url in found_urls {
                            if let Some(domain) = url.domain() {
                                if domain == base_domain {
                                    let url_str = url.to_string();
                                    if !visited.contains(&url_str) && !in_flight.contains(&url_str) {
                                        urls_to_visit.push_back(url);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("  [에러] {} 페이지를 가져올 수 없습니다: {}", cleaned_str, e);
                    }
                }
            }
        }
    }

    Ok(visited)
}

fn generate_xml(
    options: &SitemapOptions,
    urls: &HashSet<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(&options.output_file)?;
    let mut writer = Writer::new(BufWriter::new(file));

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    let mut urlset_tag = BytesStart::new("urlset");
    urlset_tag.push_attribute(("xmlns", "http://www.sitemaps.org/schemas/sitemap/0.9"));
    writer.write_event(Event::Start(urlset_tag))?;

    for url in urls {
        writer.write_event(Event::Start(BytesStart::new("url")))?;
        writer.write_event(Event::Start(BytesStart::new("loc")))?;
        writer.write_event(Event::Text(BytesText::new(url)))?;
        writer.write_event(Event::End(BytesEnd::new("loc")))?;

        if let LastmodOption::Exact(date) = &options.lastmod_option {
            writer.write_event(Event::Start(BytesStart::new("lastmod")))?;
            writer.write_event(Event::Text(BytesText::new(date)))?;
            writer.write_event(Event::End(BytesEnd::new("lastmod")))?;
        }

        writer.write_event(Event::Start(BytesStart::new("changefreq")))?;
        writer.write_event(Event::Text(BytesText::new(&options.changefreq)))?;
        writer.write_event(Event::End(BytesEnd::new("changefreq")))?;

        if let Some(p) = options.priority {
            writer.write_event(Event::Start(BytesStart::new("priority")))?;
            writer.write_event(Event::Text(BytesText::new(&p.to_string())))?;
            writer.write_event(Event::End(BytesEnd::new("priority")))?;
        }
        writer.write_event(Event::End(BytesEnd::new("url")))?;
    }

    writer.write_event(Event::End(BytesEnd::new("urlset")))?;
    println!("\n'{}' 생성이 완료되었습니다!", options.output_file);
    Ok(())
}


// --- 대화형으로 변경된 main 함수 ---
#[tokio::main]
async fn main() {
    let theme = ColorfulTheme::default();
    println!("🚀 XML Sitemap 생성기 (대화형 모드)");
    println!("------------------------------------");

    // 1. URL 입력받기
    let base_url: String = Input::with_theme(&theme)
        .with_prompt("1. 사이트맵을 생성할 URL을 입력하세요 (예: https://example.com)")
        .validate_with(|input: &String| -> Result<(), &str> {
            if Url::parse(input).is_ok() {
                Ok(())
            } else {
                Err("유효한 URL 형식이 아닙니다. 'http://' 또는 'https://'를 포함해주세요.")
            }
        })
        .interact_text()
        .unwrap();

    // 2. Page changing frequency 선택
    let changefreqs = &["daily", "weekly", "monthly", "yearly", "never"];
    let changefreq_selection = Select::with_theme(&theme)
        .with_prompt("2. 페이지 변경 빈도를 선택하세요")
        .items(changefreqs)
        .default(1) // 'weekly'를 기본값으로
        .interact()
        .unwrap();
    let changefreq = changefreqs[changefreq_selection].to_string();

    // 3. Last modified date 선택
    let lastmod_options = &["지정 안함 (Don't specify)", "정확한 날짜 사용 (Use exact value)"];
    let lastmod_selection = Select::with_theme(&theme)
        .with_prompt("3. 마지막 수정일 옵션을 선택하세요")
        .items(lastmod_options)
        .default(0)
        .interact()
        .unwrap();
    
    let lastmod_option = if lastmod_selection == 1 {
        let date_str: String = Input::with_theme(&theme)
            .with_prompt("  - 사용할 날짜를 입력하세요 (YYYY-MM-DD 형식)")
            .interact_text()
            .unwrap();
        LastmodOption::Exact(date_str)
    } else {
        LastmodOption::None
    };

    // 4. Page priority 선택
    let priority_options = &["지정 안함 (Don't specify)", "정확한 값 사용 (Use exact value)"];
    let priority_selection = Select::with_theme(&theme)
        .with_prompt("4. 페이지 우선순위 옵션을 선택하세요")
        .items(priority_options)
        .default(1)
        .interact()
        .unwrap();

    let priority: Option<f32> = if priority_selection == 1 {
        Some(Input::with_theme(&theme)
            .with_prompt("  - 우선순위 값을 입력하세요 (0.0 ~ 1.0)")
            .default(0.5)
            .validate_with(|input: &f32| -> Result<(), &str> {
                if (0.0..=1.0).contains(input) {
                    Ok(())
                } else {
                    Err("값은 0.0과 1.0 사이여야 합니다.")
                }
            })
            .interact()
            .unwrap())
    } else {
        None
    };

    // 5. 세션 파라미터 입력
    let session_params_str: String = Input::with_theme(&theme)
        .with_prompt("5. URL에서 제거할 세션 파라미터를 입력하세요 (쉼표로 구분)")
        .default("sid,phpsessid".to_string())
        .interact_text()
        .unwrap();
    let session_params_to_remove: Vec<String> = session_params_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // 모든 옵션을 구조체에 담기
    let options = SitemapOptions {
        base_url: base_url.clone(),
        output_file: "sitemap.xml".to_string(),
        changefreq,
        lastmod_option,
        priority,
        session_params_to_remove,
    };

    println!("\n------------------------------------");
    println!("설정이 완료되었습니다. 크롤링을 시작합니다.");

    // 크롤링 및 파일 생성 실행
    let start_url = Url::parse(&base_url).unwrap();
    match crawl_site(&start_url, &options.session_params_to_remove).await {
        Ok(found_urls) => {
            println!("\n총 {}개의 페이지를 찾았습니다. '{}' 파일을 생성합니다...", found_urls.len(), options.output_file);
            if let Err(e) = generate_xml(&options, &found_urls) {
                eprintln!("XML 파일 생성 중 오류가 발생했습니다: {}", e);
            }
        }
        Err(e) => {
            eprintln!("크롤링 중 오류가 발생했습니다: {}", e);
        }
    }
}