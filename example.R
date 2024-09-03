library(asynchttp)

client <- new_http_client(1)

do_request <- function() {
  client |>
    request_builder("https://edge.edx.org/c4x/BITSPilani/EEE231/asset/8086_family_Users_Manual_1_.pdf") |>
    set_method("GET") |>
    set_header("X-HttpStatus-Sleep", "1000") |>
    send_request() |>
    stream_body(callback = \(bytes) {
      print(Sys.time())
      print(length(bytes))
    })
}

do_request()
do_request()

repeat { later::run_now() }
