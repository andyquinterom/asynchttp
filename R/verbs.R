#' @export
new_http_client <- function(concurrency = 4) {
  HttpClient$new(concurrency)
}

#' @export
request_builder <- function(client, url) {
  RequestBuilder$from_client(client, url)
}

#' TODO: Handle other types
#' @export
set_header <- function(req, header_name, header_value) {
  req$set_header(header_name, header_value)
  return(invisible(req))
}

#' @export
set_method <- function(req, method) {
  req$set_method(method)
  return(invisible(req))
}

#' @export
body_raw <- function(req, contents) {
  req$set_body_raw(as.raw(contents))
  return(invisible(req))
}

POLL_INTERVAL <- 0.005 # nolint: object_name_linter.

#' @export
send_request <- function(req, poll_interval = POLL_INTERVAL) {

  eventual_response <- req$send_request()

  promises::promise(function(resolve, reject) {

    poll_recursive <- function() {
      is_ready <- eventual_response$poll()
      if (is_ready) {
        resolve(eventual_response)
      } else {
        later::later(poll_recursive, poll_interval)
      }
    }

    poll_recursive()

  })
}

get_body_string_onFulfilled <- function(resp) {
  resp$get_content_string()
}

#' @export
get_body_string <- function(resp) {
  promises::then(
    resp,
    onFulfilled = get_body_string_onFulfilled
  )
}

#' @export
attach_callback <- function(stream_promise, callback = function(bytes) {}, .poll_interval = POLL_INTERVAL) {

  #TODO: Add asserts

  promises::then(
    stream_promise,
    onFulfilled = \(stream) {
      promises::promise(function(resolve, reject) {

        poll_recursive <- function() {
          is_done <- stream$is_done()
          raw_bytes <- stream$poll()
          read_bytes <- length(raw_bytes)
          if (is_done && read_bytes == 0) {
            resolve(NULL)
          } else {
            if (read_bytes > 0) {
              callback(raw_bytes)
            }
            later::later(poll_recursive, .poll_interval)
          }
        }

        poll_recursive()

      })
    }
  )

}

#' @export
collect_string <- function(stream_promise, .poll_interval = POLL_INTERVAL) {

  #TODO: Add asserts

  promises::then(
    stream_promise,
    onFulfilled = \(stream) {
      promises::promise(function(resolve, reject) {

        poll_recursive <- function() {
          is_done <- stream$is_done()
          if (is_done) {
            resolve(stream$collect_string())
          } else {
            later::later(poll_recursive, .poll_interval)
          }
        }

        poll_recursive()

      })
    }
  )

}

#' @export
collect_json <- function(stream_promise, .poll_interval = POLL_INTERVAL) {

  #TODO: Add asserts

  promises::then(
    stream_promise,
    onFulfilled = \(stream) {
      promises::promise(function(resolve, reject) {

        poll_recursive <- function() {
          is_done <- stream$is_done()
          if (is_done) {
            resolve(stream$collect_json())
          } else {
            later::later(poll_recursive, .poll_interval)
          }
        }

        poll_recursive()

      })
    }
  )

}

#' @export
get_body_stream <- function(resp, callback = function(bytes) {}, .poll_interval = POLL_INTERVAL) {
  promises::then(
    resp,
    onFulfilled = \(resp) resp$get_body_stream()
  )
}

#' @export
redirect_body_stream <- function(resp, path, .poll_interval = POLL_INTERVAL) {
  promises::then(
    resp,
    onFulfilled = \(resp) {
      resp$redirect_body_stream(path)
    }
  )
}
