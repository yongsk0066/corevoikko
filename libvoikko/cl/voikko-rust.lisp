;;;; voikko-rust.lisp — CFFI binding for voikko_ffi (Rust cdylib)
;;;;
;;;; Single-file Common Lisp wrapper for the Rust Voikko Finnish NLP library.
;;;; Load with: (load "voikko-rust.lisp") after (ql:quickload :cffi)
;;;;
;;;; The Rust FFI accepts raw dictionary bytes (mor.vfst, autocorr.vfst)
;;;; rather than file paths, so initialization reads files into octet vectors
;;;; and passes them to voikko_new.
;;;;
;;;; Copyright (C) 2026 Corevoikko contributors
;;;; License: MPL 1.1 / GPL 2+ / LGPL 2.1+ (tri-license)

;;; ════════════════════════════════════════════════════════════════
;;; Package
;;; ════════════════════════════════════════════════════════════════

(defpackage #:voikko-rust
  (:use #:cl #:cffi)
  (:export
   ;; Lifecycle
   #:voikko-new
   #:voikko-free
   #:with-voikko
   ;; Core API
   #:voikko-spell
   #:voikko-suggest
   #:voikko-analyze
   #:voikko-hyphenate
   #:voikko-insert-hyphens
   #:voikko-grammar-errors
   #:voikko-tokens
   #:voikko-sentences
   ;; Utility
   #:voikko-version
   #:voikko-attribute-values
   ;; Option setters
   #:voikko-set-ignore-dot
   #:voikko-set-ignore-numbers
   #:voikko-set-ignore-uppercase
   #:voikko-set-no-ugly-hyphenation
   #:voikko-set-accept-first-uppercase
   #:voikko-set-accept-all-uppercase
   #:voikko-set-ocr-suggestions
   #:voikko-set-ignore-nonwords
   #:voikko-set-accept-extra-hyphens
   #:voikko-set-accept-missing-hyphens
   #:voikko-set-accept-titles-in-gc
   #:voikko-set-accept-unfinished-paragraphs-in-gc
   #:voikko-set-hyphenate-unknown-words
   #:voikko-set-accept-bulleted-lists-in-gc
   #:voikko-set-min-hyphenated-word-length
   #:voikko-set-max-suggestions
   #:voikko-set-speller-cache-size
   ;; Token / sentence type constants
   #:+token-none+ #:+token-word+ #:+token-punctuation+
   #:+token-whitespace+ #:+token-unknown+
   #:+sentence-none+ #:+sentence-no-start+
   #:+sentence-probable+ #:+sentence-possible+
   ;; Conditions
   #:voikko-error))

(in-package #:voikko-rust)

;;; ════════════════════════════════════════════════════════════════
;;; Library loading
;;; ════════════════════════════════════════════════════════════════

(define-foreign-library libvoikko-ffi
  (:darwin "libvoikko_ffi.dylib")
  (:unix "libvoikko_ffi.so")
  (:windows "voikko_ffi.dll")
  (t (:default "libvoikko_ffi")))

(eval-when (:load-toplevel :execute)
  (with-simple-restart (skip "Skip loading libvoikko_ffi.")
    (load-foreign-library 'libvoikko-ffi)))

;;; ════════════════════════════════════════════════════════════════
;;; Conditions
;;; ════════════════════════════════════════════════════════════════

(define-condition voikko-error (error)
  ((message :initarg :message :reader voikko-error-message))
  (:report (lambda (c s) (format s "Voikko error: ~A" (voikko-error-message c)))))

;;; ════════════════════════════════════════════════════════════════
;;; Constants
;;; ════════════════════════════════════════════════════════════════

;; Token types returned by voikko_tokens
(defconstant +token-none+        0)
(defconstant +token-word+        1)
(defconstant +token-punctuation+ 2)
(defconstant +token-whitespace+  3)
(defconstant +token-unknown+     4)

;; Sentence types returned by voikko_sentences
(defconstant +sentence-none+     0)
(defconstant +sentence-no-start+ 1)
(defconstant +sentence-probable+ 2)
(defconstant +sentence-possible+ 3)

;;; ════════════════════════════════════════════════════════════════
;;; C struct types
;;; ════════════════════════════════════════════════════════════════

(defcstruct voikko-analysis
  (keys   :pointer)   ; char** NULL-terminated
  (values :pointer))  ; char** NULL-terminated

(defcstruct voikko-analysis-array
  (analyses :pointer) ; VoikkoAnalysis*
  (count    :size))

(defcstruct voikko-grammar-error
  (error-code        :int)
  (start-pos         :size)
  (error-len         :size)
  (short-description :pointer)  ; char*
  (suggestions       :pointer)) ; char** NULL-terminated

(defcstruct voikko-grammar-error-array
  (errors :pointer) ; VoikkoGrammarError*
  (count  :size))

(defcstruct voikko-token
  (token-type :int)
  (text       :pointer) ; char*
  (position   :size))

(defcstruct voikko-token-array
  (tokens :pointer) ; VoikkoToken*
  (count  :size))

(defcstruct voikko-sentence
  (sentence-type :int)
  (sentence-len  :size))

(defcstruct voikko-sentence-array
  (sentences :pointer) ; VoikkoSentence*
  (count     :size))

;;; ════════════════════════════════════════════════════════════════
;;; Low-level FFI declarations
;;; ════════════════════════════════════════════════════════════════

;; -- Handle lifecycle --

(defcfun ("voikko_new" %voikko-new) :pointer
  (mor-data      :pointer)
  (mor-len       :size)
  (autocorr-data :pointer)
  (autocorr-len  :size)
  (error-out     :pointer))

(defcfun ("voikko_free" %voikko-free) :void
  (handle :pointer))

;; -- Spell checking --

(defcfun ("voikko_spell" %voikko-spell) :int
  (handle :pointer)
  (word   :string))

(defcfun ("voikko_suggest" %voikko-suggest) :pointer
  (handle :pointer)
  (word   :string))

;; -- Morphological analysis --

(defcfun ("voikko_analyze" %voikko-analyze) (:struct voikko-analysis-array)
  (handle :pointer)
  (word   :string))

(defcfun ("voikko_free_analyses" %voikko-free-analyses) :void
  (arr (:struct voikko-analysis-array)))

;; -- Hyphenation --

(defcfun ("voikko_hyphenate" %voikko-hyphenate) :pointer
  (handle :pointer)
  (word   :string))

(defcfun ("voikko_insert_hyphens" %voikko-insert-hyphens) :pointer
  (handle                :pointer)
  (word                  :string)
  (separator             :string)
  (allow-context-changes :int))

;; -- Grammar checking --

(defcfun ("voikko_grammar_errors" %voikko-grammar-errors)
    (:struct voikko-grammar-error-array)
  (handle   :pointer)
  (text     :string)
  (language :string))

(defcfun ("voikko_free_grammar_errors" %voikko-free-grammar-errors) :void
  (arr (:struct voikko-grammar-error-array)))

;; -- Tokenization --

(defcfun ("voikko_tokens" %voikko-tokens) (:struct voikko-token-array)
  (handle :pointer)
  (text   :string))

(defcfun ("voikko_free_tokens" %voikko-free-tokens) :void
  (arr (:struct voikko-token-array)))

;; -- Sentence detection --

(defcfun ("voikko_sentences" %voikko-sentences) (:struct voikko-sentence-array)
  (handle :pointer)
  (text   :string))

(defcfun ("voikko_free_sentences" %voikko-free-sentences) :void
  (arr (:struct voikko-sentence-array)))

;; -- Option setters --

(defcfun ("voikko_set_ignore_dot" %set-ignore-dot) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_ignore_numbers" %set-ignore-numbers) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_ignore_uppercase" %set-ignore-uppercase) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_no_ugly_hyphenation" %set-no-ugly-hyphenation) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_accept_first_uppercase" %set-accept-first-uppercase) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_accept_all_uppercase" %set-accept-all-uppercase) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_ocr_suggestions" %set-ocr-suggestions) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_ignore_nonwords" %set-ignore-nonwords) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_accept_extra_hyphens" %set-accept-extra-hyphens) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_accept_missing_hyphens" %set-accept-missing-hyphens) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_accept_titles_in_gc" %set-accept-titles-in-gc) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_accept_unfinished_paragraphs_in_gc"
          %set-accept-unfinished-paragraphs-in-gc) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_hyphenate_unknown_words" %set-hyphenate-unknown-words) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_accept_bulleted_lists_in_gc"
          %set-accept-bulleted-lists-in-gc) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_min_hyphenated_word_length"
          %set-min-hyphenated-word-length) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_max_suggestions" %set-max-suggestions) :void
  (handle :pointer) (value :int))
(defcfun ("voikko_set_speller_cache_size" %set-speller-cache-size) :void
  (handle :pointer) (value :int))

;; -- Utility --

(defcfun ("voikko_version" %voikko-version) :string)

(defcfun ("voikko_attribute_values" %voikko-attribute-values) :pointer
  (name :string))

;; -- Memory management --

(defcfun ("voikko_free_str" %voikko-free-str) :void
  (s :pointer))

(defcfun ("voikko_free_str_array" %voikko-free-str-array) :void
  (arr :pointer))

;;; ════════════════════════════════════════════════════════════════
;;; Internal helpers
;;; ════════════════════════════════════════════════════════════════

(defun read-file-octets (path)
  "Read PATH into an octet vector."
  (with-open-file (in path :element-type '(unsigned-byte 8))
    (let* ((len (file-length in))
           (buf (make-array len :element-type '(unsigned-byte 8))))
      (read-sequence buf in)
      buf)))

(defun null-pointer-p* (ptr)
  "Return T if PTR is null or not a pointer."
  (or (not (pointerp ptr))
      (null-pointer-p ptr)))

(defun collect-null-terminated-strings (ptr)
  "Read a NULL-terminated char** array into a list of Lisp strings.
Does NOT free the array."
  (when (and (pointerp ptr) (not (null-pointer-p ptr)))
    (loop :for i :from 0
          :for p := (mem-aref ptr :pointer i)
          :until (null-pointer-p p)
          :collect (foreign-string-to-lisp p))))

(defun bool->int (value)
  "Convert a generalized boolean to C int (0/1)."
  (if value 1 0))

;;; ════════════════════════════════════════════════════════════════
;;; Handle lifecycle
;;; ════════════════════════════════════════════════════════════════

(defun voikko-new (dict-path)
  "Create a Voikko handle by loading dictionary files from DICT-PATH.

DICT-PATH is a directory pathname (string or pathname) containing mor.vfst
and optionally autocorr.vfst.  A V5 directory structure
\(DICT-PATH/5/mor-standard/\) is also recognized.

Returns a foreign pointer (the opaque handle).
Signals VOIKKO-ERROR on failure."
  (let* ((dir (pathname dict-path))
         (mor-path (merge-pathnames "mor.vfst" dir))
         ;; Auto-detect V5 structure
         (mor-path (if (probe-file mor-path)
                       mor-path
                       (let ((v5 (merge-pathnames
                                  "5/mor-standard/mor.vfst" dir)))
                         (if (probe-file v5)
                             v5
                             (error 'voikko-error
                                    :message (format nil
                                              "mor.vfst not found in ~A"
                                              dict-path))))))
         (base-dir (make-pathname :directory (pathname-directory mor-path)))
         (autocorr-path (merge-pathnames "autocorr.vfst" base-dir))
         (mor-octets (read-file-octets mor-path))
         (autocorr-octets (when (probe-file autocorr-path)
                            (read-file-octets autocorr-path))))
    (with-foreign-objects ((error-out :pointer))
      (setf (mem-ref error-out :pointer) (null-pointer))
      (let ((handle
              (with-foreign-array (mor-buf mor-octets
                                  `(:array :uint8 ,(length mor-octets)))
                (if autocorr-octets
                    (with-foreign-array (ac-buf autocorr-octets
                                        `(:array :uint8 ,(length autocorr-octets)))
                      (%voikko-new mor-buf (length mor-octets)
                                   ac-buf (length autocorr-octets)
                                   error-out))
                    (%voikko-new mor-buf (length mor-octets)
                                 (null-pointer) 0
                                 error-out)))))
        (when (null-pointer-p* handle)
          (let* ((err-ptr (mem-ref error-out :pointer))
                 (msg (if (null-pointer-p* err-ptr)
                          "unknown error"
                          (prog1 (foreign-string-to-lisp err-ptr)
                            (%voikko-free-str err-ptr)))))
            (error 'voikko-error
                   :message (format nil "Failed to initialize Voikko: ~A" msg))))
        handle))))

(defun voikko-free (handle)
  "Free a Voikko handle. Safe to call with NIL or null pointer."
  (when (and handle (pointerp handle) (not (null-pointer-p handle)))
    (%voikko-free handle)))

(defmacro with-voikko ((var dict-path) &body body)
  "Create a Voikko handle bound to VAR, execute BODY, then free the handle.

DICT-PATH is a directory containing mor.vfst (see VOIKKO-NEW).
Returns the values of the last form in BODY."
  (let ((handle (gensym "HANDLE")))
    `(let* ((,handle (voikko-new ,dict-path))
            (,var ,handle))
       (declare (ignorable ,var))
       (unwind-protect (progn ,@body)
         (voikko-free ,handle)))))

;;; ════════════════════════════════════════════════════════════════
;;; Spell checking
;;; ════════════════════════════════════════════════════════════════

(defun voikko-spell (handle word)
  "Check spelling of WORD. Returns T if correct, NIL otherwise.
Returns NIL on error (null handle or invalid input)."
  (check-type word string)
  (let ((result (%voikko-spell handle word)))
    (= result 1)))

(defun voikko-suggest (handle word)
  "Return a list of spelling suggestions for WORD.
Returns NIL if no suggestions are available."
  (check-type word string)
  (let ((ptr (%voikko-suggest handle word)))
    (when (and (pointerp ptr) (not (null-pointer-p ptr)))
      (unwind-protect (collect-null-terminated-strings ptr)
        (%voikko-free-str-array ptr)))))

;;; ════════════════════════════════════════════════════════════════
;;; Morphological analysis
;;; ════════════════════════════════════════════════════════════════

(defun voikko-analyze (handle word)
  "Perform morphological analysis of WORD.

Returns a list of analyses. Each analysis is an alist of (KEY . VALUE)
string pairs representing morphological attributes."
  (check-type word string)
  (let ((arr (%voikko-analyze handle word)))
    ;; arr is a plist: (ANALYSES ptr COUNT n)
    (let ((analyses-ptr (getf arr 'analyses))
          (count (getf arr 'count)))
      (when (or (null-pointer-p* analyses-ptr) (zerop count))
        (return-from voikko-analyze nil))
      (unwind-protect
           (loop :for i :below count
                 :for a-ptr := (mem-aptr analyses-ptr
                                         '(:struct voikko-analysis) i)
                 :collect
                 (let ((keys-ptr (foreign-slot-value
                                  a-ptr '(:struct voikko-analysis) 'keys))
                       (vals-ptr (foreign-slot-value
                                  a-ptr '(:struct voikko-analysis) 'values)))
                   (when (and (not (null-pointer-p* keys-ptr))
                              (not (null-pointer-p* vals-ptr)))
                     (loop :for j :from 0
                           :for k := (mem-aref keys-ptr :pointer j)
                           :until (null-pointer-p k)
                           :for v := (mem-aref vals-ptr :pointer j)
                           :collect (cons (foreign-string-to-lisp k)
                                         (foreign-string-to-lisp v))))))
        (%voikko-free-analyses arr)))))

;;; ════════════════════════════════════════════════════════════════
;;; Hyphenation
;;; ════════════════════════════════════════════════════════════════

(defun voikko-hyphenate (handle word)
  "Return the hyphenation pattern string for WORD.

The pattern has the same length as WORD:
  #\\Space = no hyphenation point
  #\\-     = hyphenation point (character preserved)
  #\\=     = hyphenation point (character replaced by hyphen)

Returns NIL on error."
  (check-type word string)
  (let ((ptr (%voikko-hyphenate handle word)))
    (when (and (pointerp ptr) (not (null-pointer-p ptr)))
      (unwind-protect (foreign-string-to-lisp ptr)
        (%voikko-free-str ptr)))))

(defun voikko-insert-hyphens (handle word &key (separator "-")
                                               (allow-context-changes nil))
  "Hyphenate WORD by inserting SEPARATOR between syllables.

When ALLOW-CONTEXT-CHANGES is non-nil, hyphens may alter the word form.
Returns the hyphenated string, or NIL on error."
  (check-type word string)
  (check-type separator string)
  (let ((ptr (%voikko-insert-hyphens handle word separator
                                      (bool->int allow-context-changes))))
    (when (and (pointerp ptr) (not (null-pointer-p ptr)))
      (unwind-protect (foreign-string-to-lisp ptr)
        (%voikko-free-str ptr)))))

;;; ════════════════════════════════════════════════════════════════
;;; Grammar checking
;;; ════════════════════════════════════════════════════════════════

(defun voikko-grammar-errors (handle text &key (language "fi"))
  "Check TEXT for grammar errors.

Returns a list of plists, each with keys:
  :ERROR-CODE  — integer error code
  :START-POS   — character offset
  :ERROR-LEN   — length of the error span
  :DESCRIPTION — short description string
  :SUGGESTIONS — list of suggested corrections"
  (check-type text string)
  (check-type language string)
  (let ((arr (%voikko-grammar-errors handle text language)))
    (let ((errors-ptr (getf arr 'errors))
          (count (getf arr 'count)))
      (when (or (null-pointer-p* errors-ptr) (zerop count))
        (return-from voikko-grammar-errors nil))
      (unwind-protect
           (loop :for i :below count
                 :for e-ptr := (mem-aptr errors-ptr
                                         '(:struct voikko-grammar-error) i)
                 :collect
                 (let ((desc-ptr (foreign-slot-value
                                  e-ptr '(:struct voikko-grammar-error)
                                  'short-description))
                       (sugg-ptr (foreign-slot-value
                                  e-ptr '(:struct voikko-grammar-error)
                                  'suggestions)))
                   (list :error-code
                         (foreign-slot-value
                          e-ptr '(:struct voikko-grammar-error) 'error-code)
                         :start-pos
                         (foreign-slot-value
                          e-ptr '(:struct voikko-grammar-error) 'start-pos)
                         :error-len
                         (foreign-slot-value
                          e-ptr '(:struct voikko-grammar-error) 'error-len)
                         :description
                         (if (null-pointer-p* desc-ptr)
                             ""
                             (foreign-string-to-lisp desc-ptr))
                         :suggestions
                         (if (null-pointer-p* sugg-ptr)
                             nil
                             (collect-null-terminated-strings sugg-ptr)))))
        (%voikko-free-grammar-errors arr)))))

;;; ════════════════════════════════════════════════════════════════
;;; Tokenization
;;; ════════════════════════════════════════════════════════════════

(defun voikko-tokens (handle text)
  "Tokenize TEXT.

Returns a list of plists, each with keys:
  :TOKEN-TYPE — integer (see +TOKEN-*+ constants)
  :TEXT       — the token string
  :POSITION   — character offset in the original text"
  (check-type text string)
  (let ((arr (%voikko-tokens handle text)))
    (let ((tokens-ptr (getf arr 'tokens))
          (count (getf arr 'count)))
      (when (or (null-pointer-p* tokens-ptr) (zerop count))
        (return-from voikko-tokens nil))
      (unwind-protect
           (loop :for i :below count
                 :for t-ptr := (mem-aptr tokens-ptr
                                         '(:struct voikko-token) i)
                 :collect
                 (let ((txt-ptr (foreign-slot-value
                                 t-ptr '(:struct voikko-token) 'text)))
                   (list :token-type
                         (foreign-slot-value
                          t-ptr '(:struct voikko-token) 'token-type)
                         :text
                         (if (null-pointer-p* txt-ptr)
                             ""
                             (foreign-string-to-lisp txt-ptr))
                         :position
                         (foreign-slot-value
                          t-ptr '(:struct voikko-token) 'position))))
        (%voikko-free-tokens arr)))))

;;; ════════════════════════════════════════════════════════════════
;;; Sentence detection
;;; ════════════════════════════════════════════════════════════════

(defun voikko-sentences (handle text)
  "Detect sentence boundaries in TEXT.

Returns a list of plists, each with keys:
  :SENTENCE-TYPE — integer (see +SENTENCE-*+ constants)
  :LENGTH        — length of the sentence in characters"
  (check-type text string)
  (let ((arr (%voikko-sentences handle text)))
    (let ((sentences-ptr (getf arr 'sentences))
          (count (getf arr 'count)))
      (when (or (null-pointer-p* sentences-ptr) (zerop count))
        (return-from voikko-sentences nil))
      (unwind-protect
           (loop :for i :below count
                 :for s-ptr := (mem-aptr sentences-ptr
                                         '(:struct voikko-sentence) i)
                 :collect
                 (list :sentence-type
                       (foreign-slot-value
                        s-ptr '(:struct voikko-sentence) 'sentence-type)
                       :length
                       (foreign-slot-value
                        s-ptr '(:struct voikko-sentence) 'sentence-len)))
        (%voikko-free-sentences arr)))))

;;; ════════════════════════════════════════════════════════════════
;;; Option setters
;;; ════════════════════════════════════════════════════════════════

(defun voikko-set-ignore-dot (handle value)
  "Set ignore-dot option. VALUE is a generalized boolean."
  (%set-ignore-dot handle (bool->int value)))

(defun voikko-set-ignore-numbers (handle value)
  "Set ignore-numbers option. VALUE is a generalized boolean."
  (%set-ignore-numbers handle (bool->int value)))

(defun voikko-set-ignore-uppercase (handle value)
  "Set ignore-uppercase option. VALUE is a generalized boolean."
  (%set-ignore-uppercase handle (bool->int value)))

(defun voikko-set-no-ugly-hyphenation (handle value)
  "Set no-ugly-hyphenation option. VALUE is a generalized boolean."
  (%set-no-ugly-hyphenation handle (bool->int value)))

(defun voikko-set-accept-first-uppercase (handle value)
  "Set accept-first-uppercase option. VALUE is a generalized boolean."
  (%set-accept-first-uppercase handle (bool->int value)))

(defun voikko-set-accept-all-uppercase (handle value)
  "Set accept-all-uppercase option. VALUE is a generalized boolean."
  (%set-accept-all-uppercase handle (bool->int value)))

(defun voikko-set-ocr-suggestions (handle value)
  "Set OCR suggestions mode. VALUE is a generalized boolean."
  (%set-ocr-suggestions handle (bool->int value)))

(defun voikko-set-ignore-nonwords (handle value)
  "Set ignore-nonwords option. VALUE is a generalized boolean."
  (%set-ignore-nonwords handle (bool->int value)))

(defun voikko-set-accept-extra-hyphens (handle value)
  "Set accept-extra-hyphens option. VALUE is a generalized boolean."
  (%set-accept-extra-hyphens handle (bool->int value)))

(defun voikko-set-accept-missing-hyphens (handle value)
  "Set accept-missing-hyphens option. VALUE is a generalized boolean."
  (%set-accept-missing-hyphens handle (bool->int value)))

(defun voikko-set-accept-titles-in-gc (handle value)
  "Set accept-titles-in-gc option. VALUE is a generalized boolean."
  (%set-accept-titles-in-gc handle (bool->int value)))

(defun voikko-set-accept-unfinished-paragraphs-in-gc (handle value)
  "Set accept-unfinished-paragraphs-in-gc option. VALUE is a generalized boolean."
  (%set-accept-unfinished-paragraphs-in-gc handle (bool->int value)))

(defun voikko-set-hyphenate-unknown-words (handle value)
  "Set hyphenate-unknown-words option. VALUE is a generalized boolean."
  (%set-hyphenate-unknown-words handle (bool->int value)))

(defun voikko-set-accept-bulleted-lists-in-gc (handle value)
  "Set accept-bulleted-lists-in-gc option. VALUE is a generalized boolean."
  (%set-accept-bulleted-lists-in-gc handle (bool->int value)))

(defun voikko-set-min-hyphenated-word-length (handle value)
  "Set minimum hyphenated word length. VALUE is a positive integer."
  (check-type value integer)
  (%set-min-hyphenated-word-length handle value))

(defun voikko-set-max-suggestions (handle value)
  "Set maximum number of spelling suggestions. VALUE is a positive integer."
  (check-type value integer)
  (%set-max-suggestions handle value))

(defun voikko-set-speller-cache-size (handle value)
  "Set speller cache size. VALUE is an integer (-1 for no cache, >= 0 for size)."
  (check-type value integer)
  (%set-speller-cache-size handle value))

;;; ════════════════════════════════════════════════════════════════
;;; Utility
;;; ════════════════════════════════════════════════════════════════

(defun voikko-version ()
  "Return the Voikko library version string.
The returned string is static and must NOT be freed."
  (%voikko-version))

(defun voikko-attribute-values (name)
  "Return a list of valid values for the morphological attribute NAME.
Returns NIL if the attribute is not recognized.
The returned pointers are static and must NOT be freed."
  (check-type name string)
  (let ((ptr (%voikko-attribute-values name)))
    (when (and (pointerp ptr) (not (null-pointer-p ptr)))
      (collect-null-terminated-strings ptr))))

;;; ════════════════════════════════════════════════════════════════
;;; Usage example (commented out)
;;; ════════════════════════════════════════════════════════════════

#|
;; Load CFFI first:
;;   (ql:quickload :cffi)
;;
;; Set library search path if needed:
;;   (pushnew #P"/path/to/target/release/"
;;            cffi:*foreign-library-directories* :test #'equal)
;;
;; Load this file:
;;   (load "voikko-rust.lisp")
;;
;; Use:
(voikko-rust:with-voikko (v "/path/to/voikko-fi/vvfst/")
  (format t "Version: ~A~%" (voikko-rust:voikko-version))
  (format t "Spell 'kissa': ~A~%" (voikko-rust:voikko-spell v "kissa"))
  (format t "Suggest 'kisss': ~A~%" (voikko-rust:voikko-suggest v "kisss"))
  (format t "Analyze 'kissalla': ~A~%" (voikko-rust:voikko-analyze v "kissalla"))
  (format t "Hyphenate 'yhdyssana': ~A~%"
          (voikko-rust:voikko-insert-hyphens v "yhdyssana"))
  (format t "Tokens: ~A~%" (voikko-rust:voikko-tokens v "Kissa istuu."))
  (format t "Sentences: ~A~%" (voikko-rust:voikko-sentences v "Kissa istuu. Koira makaa.")))
|#
