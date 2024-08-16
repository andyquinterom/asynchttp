ASYNC_HTTP_ENV <- new.env(parent = emptyenv())

#' @export
post <- function(url) {
  RequestBuilder$new("post", url)
}

#' @export
get <- function(url) {
  RequestBuilder$new("get", url)
}

#' @export
put <- function(url) {
  RequestBuilder$new("put", url)
}

#' @export
delete <- function(url) {
  RequestBuilder$new("delete", url)
}

#' @export
set_header <- function(req, header_name, header_value) {
  req$set_header(header_name, header_value)
}

#' @export
body_raw <- function(req, contents) {
  req$set_body_raw(as.raw(contents))
}

POLL_INTERVAL <- 0.005

#' @export
send_request <- function(req) {

  if (is.null(ASYNC_HTTP_ENV$CLIENT)) {
    ASYNC_HTTP_ENV$CLIENT <- HttpClient$new(4L)
  }

  eventual_response <- req$send_request(ASYNC_HTTP_ENV$CLIENT)

  promises::promise(function(resolve, reject) {

    poll_recursive <- function() {
      is_ready <- eventual_response$poll()
      if (is_ready) {
        string_contents <- eventual_response$get_content_string()
        resolve(string_contents)
      }
      later::later(poll_recursive, POLL_INTERVAL)
    }

    poll_recursive()

  })
}
