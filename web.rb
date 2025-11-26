#!/usr/bin/env ruby

require "okay/http"
require "pg"
require "sinatra"

REQUIRED_REVIEWS = 3

def pg_password
  File.read('pg-env').split("\n").map {|l| l.split("=", 2) }.to_h["PGPASSWORD"]
end

$conn = PG.connect(host: 'localhost', dbname: 'rubygems', user: "postgres", password: pg_password)

def mark_approved(gem_name, gem_version)
  $conn.exec_params("UPDATE push_reviews SET (reviewed, approvals) = (true, approvals + 1)
                    WHERE gem_name = $1 AND gem_version = $2",
                    [gem_name, gem_version])
end

def mark_rejected(gem_name, gem_version)
  $conn.exec_params("UPDATE push_reviews SET (reviewed, rejections) = (true, rejections + 1)
                    WHERE gem_name = $1 AND gem_version = $2",
                    [gem_name, gem_version])
end

def mark_skipped(gem_name, gem_version)
  $conn.exec_params("UPDATE push_reviews SET (reviewed, skips) = (true, skips + 1)
                    WHERE gem_name = $1 AND gem_version = $2",
                    [gem_name, gem_version])
end

def get_gem
  $conn.exec("
    SELECT * FROM push_reviews TABLESAMPLE BERNOULLI (30)
    WHERE ( CARDINALITY(approvals) + CARDINALITY(rejections) ) < #{REQUIRED_REVIEWS}
    ORDER BY version_created_at
    LIMIT 1
  ") do |result|
    result.map { |row|
      gem_name, gem_version, version_created_at, previous_version = row.values_at('gem_name', 'gem_version', 'version_created_at', 'previous_version')

      {
        name: gem_name,
        version: gem_version,
        created_at: version_created_at,
        previous_version: previous_version,
      }
    }.first
  end
end

def default_page(banner=nil)
  g = get_gem

  <<~EOF
  <!doctype html>
  <title>Comparing: #{g[:name]} #{g[:previous_version]} and #{g[:version]}</title>
  <style>
    main {
      font-family: sans-serif;
      max-width: 100ch;
      margin: auto;
    }
    input[name=approve] {
      background: green;
    }
    input[name=approve]:before {
      content '👍';
      background: green;
    }
    input[name=reject] {
      background: red;
    }
    input[name=reject]:after {
      content '👎';
      background: green;
    }
  </style>

  <main>
    <h1>Community Code Review</h1>
    <p>Hi! This project is an attempt to review every gem push to rubygems.org!</p>
    <p>Please review the following diff, and choose <em>Looks safe!</em> or <em>Needs review!</em> as appropriate.</p>
    <p>If you aren't sure, you don't want to read that much code, or it smells funny, you can click "Skip" instead.</p>
    <p>#{banner || '&nbsp;'}</p>
    <h2>#{g[:name]} #{g[:previous_version]} → #{g[:version]}</h2>
    <form method="post" action="/review">
      <p><a target="_blank" href="https://my.diffend.io/gems/#{g[:name]}/#{g[:previous_version]}/#{g[:version]}">Compare on diffend</a></p>
      <input type="hidden" name="name" value=#{g[:name].to_s.inspect}>
      <input type="hidden" name="version" value=#{g[:version].to_s.inspect}>
      <input type="submit" name="approve" value="Looks safe!">
      <input type="submit" name="reject" value="Needs review!">
      <input type="submit" name="skip" value="Skip">
    </form>
  </main>
  EOF

end

get '/' do
  default_page
end

post '/review' do
  keys = params.keys
  name = params['name']
  version = params['version']
  message =
    if keys.include?('approve')
      mark_approved(name, version)
      "Woo! Thanks for checking #{name} #{version}!"
    elsif keys.include?('reject')
      mark_rejected(name, version)
      "Oh no! Thanks for letting us know something's up with #{name} #{version}! Someone else will take a more detailed look."
    elsif keys.include?('skip')
      mark_skipped(name, version)
      "Let's try this one instead. :)"
    end

  default_page(message)
end
