#!/usr/bin/env node
// Downloads what the local engine that reads scanned pages needs (D-558,
// `src-tauri/src/source_volume/recognition_local.rs`): the models of
// RapidOCR (PP-OCRv6 small detection and recognition, the orientation
// classifier and the dictionary, the same files the `rapidocr` 3.10.0
// package carries) and ONNX Runtime 1.24.4, the library that runs them.
//
// They are not in the repository: this script puts them in `src-tauri/ocr/`
// (ignored by git), each checked against its fixed sha256. The app takes
// them from there in development, and from the installer's resources
// (`ocr/`) once they are added to it. Without them the local engine is not
// available and the app says so.
//
// Usage: node scripts/fetch-ocr-models.mjs [destination folder]
// Only Node; no package. The library is fetched for Windows x64 only for
// now (on other systems, only the models).

import { createHash } from "node:crypto"
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import { inflateRawSync } from "node:zlib"

const here = dirname(fileURLToPath(import.meta.url))
const destination = resolve(process.argv[2] ?? join(here, "..", "src-tauri", "ocr"))

const MODELS_AT = "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve"
const MODELS = [
  {
    file: "PP-OCRv6_det_small.onnx",
    url: `${MODELS_AT}/v3.9.0/onnx/PP-OCRv6/det/PP-OCRv6_det_small.onnx`,
    sha256: "090f04abcd9d9a7498bc4ebf677e4cb9bdce1fe4197ddb7e529f1ef44e1ff94f",
  },
  {
    file: "PP-OCRv6_rec_small.onnx",
    url: `${MODELS_AT}/v3.9.0/onnx/PP-OCRv6/rec/PP-OCRv6_rec_small.onnx`,
    sha256: "6f327246b50388f3c176ae304bd95767ea6dc0c9ae92153ef8cbe210b3c14884",
  },
  {
    file: "ch_ppocr_mobile_v2.0_cls_mobile.onnx",
    url: `${MODELS_AT}/v3.9.0/onnx/PP-OCRv4/cls/ch_ppocr_mobile_v2.0_cls_mobile.onnx`,
    sha256: "e47acedf663230f8863ff1ab0e64dd2d82b838fceb5957146dab185a89d6215c",
  },
  {
    file: "ppocrv6_dict.txt",
    url: `${MODELS_AT}/master/paddle/PP-OCRv6/rec/PP-OCRv6_rec_small/ppocrv6_dict.txt`,
    sha256: "b5f2bfe2bdd9448429e3e82b51c789775d9b42f2403d082b00662eb77e401c5d",
  },
]

// ONNX Runtime, CPU only, from Microsoft's release: the archive's sha256 is
// the one the release publishes.
const RUNTIME = {
  "win32-x64": {
    file: "onnxruntime.dll",
    url: "https://github.com/microsoft/onnxruntime/releases/download/v1.24.4/onnxruntime-win-x64-1.24.4.zip",
    sha256: "d2319fddfb6ea4db99ccc4b60c85c517bcd855721f5daa6a06d40d7cb2ee2357",
    entry: "onnxruntime-win-x64-1.24.4/lib/onnxruntime.dll",
  },
}

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex")

async function download(url, expected) {
  const response = await fetch(url, { redirect: "follow" })
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`)
  const bytes = Buffer.from(await response.arrayBuffer())
  const got = sha256(bytes)
  if (got !== expected) throw new Error(`${url}: sha256 ${got}, expected ${expected}`)
  return bytes
}

function save(path, bytes) {
  writeFileSync(`${path}.part`, bytes)
  renameSync(`${path}.part`, path)
}

/** The bytes of one entry of a zip archive (stored or deflated). */
function unzipEntry(zip, name) {
  let end = zip.length - 22
  while (end >= 0 && zip.readUInt32LE(end) !== 0x06054b50) end--
  if (end < 0) throw new Error("not a zip archive")
  const count = zip.readUInt16LE(end + 10)
  let at = zip.readUInt32LE(end + 16)
  for (let i = 0; i < count; i++) {
    const method = zip.readUInt16LE(at + 10)
    const size = zip.readUInt32LE(at + 20)
    const nameLength = zip.readUInt16LE(at + 28)
    const skip = nameLength + zip.readUInt16LE(at + 30) + zip.readUInt16LE(at + 32)
    const header = zip.readUInt32LE(at + 42)
    if (zip.toString("utf8", at + 46, at + 46 + nameLength) === name) {
      const data = header + 30 + zip.readUInt16LE(header + 26) + zip.readUInt16LE(header + 28)
      const body = zip.subarray(data, data + size)
      if (method === 0) return body
      if (method === 8) return inflateRawSync(body)
      throw new Error(`${name}: compression ${method} not supported`)
    }
    at += 46 + skip
  }
  throw new Error(`${name} is not in the archive`)
}

const kept = (path, expected) => existsSync(path) && sha256(readFileSync(path)) === expected

async function main() {
  const models = join(destination, "models")
  mkdirSync(models, { recursive: true })
  for (const model of MODELS) {
    const path = join(models, model.file)
    if (kept(path, model.sha256)) {
      console.log(`${model.file}: already there`)
      continue
    }
    save(path, await download(model.url, model.sha256))
    console.log(`${model.file}: downloaded`)
  }
  const runtime = RUNTIME[`${process.platform}-${process.arch}`]
  if (!runtime) {
    console.log(`ONNX Runtime: not fetched for ${process.platform}-${process.arch}; put its library in ${destination}`)
    return
  }
  const path = join(destination, runtime.file)
  if (existsSync(path)) {
    console.log(`${runtime.file}: already there`)
    return
  }
  const archive = await download(runtime.url, runtime.sha256)
  save(path, unzipEntry(archive, runtime.entry))
  console.log(`${runtime.file}: downloaded (sha256 ${sha256(readFileSync(path))})`)
}

main().catch((error) => {
  console.error(error.message)
  process.exit(1)
})
