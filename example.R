library(asynchttp)

client <- new_http_client(4)

do_request <- function() {
  client |>
    request_builder("https://httpstat.us/200") |>
    set_method("GET") |>
    set_header("X-HttpStatus-Sleep", "1000") |>
    set_header("Accept", "application/json") |>
    send_request() |>
    redirect_body_stream("hello.txt")
}

do_request()

repeat { later::run_now() }
