#!/usr/bin/env swift

import Foundation
import UniformTypeIdentifiers

func findImageFiles(in directoryURL: URL, maxDepth: Int) -> [URL] {
    var imageFiles: [URL] = []
    
    // Create enumerator
    guard let enumerator = FileManager.default.enumerator(
        at: directoryURL,
        includingPropertiesForKeys: [.typeIdentifierKey, .isRegularFileKey],
        options: [.skipsHiddenFiles, .skipsPackageDescendants]
    ) else {
        print("Unable to access directory: \(directoryURL.path)")
        return []
    }
    
    let baseDepth = directoryURL.pathComponents.count
    
    for case let fileURL as URL in enumerator {
        let currentDepth = fileURL.pathComponents.count
        let relativeDepth = currentDepth - baseDepth
        
        // Skip if depth exceeded
        if relativeDepth > maxDepth {
            enumerator.skipDescendants()
            continue
        }
        
        do {
            let resourceValues = try fileURL.resourceValues(forKeys: [.typeIdentifierKey, .isRegularFileKey])
            
            // Check if it's a regular file
            guard resourceValues.isRegularFile == true else { continue }
            
            // Check if type conforms to public.image
            if let typeIdentifier = resourceValues.typeIdentifier,
               let utType = UTType(typeIdentifier),
               utType.conforms(to: .image) {
                imageFiles.append(fileURL)
            }
        } catch {
            // Skip files we can't read
            continue
        }
    }
    
    return imageFiles
}

// Main
let args = CommandLine.arguments
guard args.count >= 2 else {
    print("Usage: test_image_search <path> [depth]")
    exit(1)
}

let path = args[1]
let depth = args.count > 2 ? Int(args[2]) ?? 1 : 1

let url = URL(fileURLWithPath: path)
let images = findImageFiles(in: url, maxDepth: depth)

print("Found \(images.count) image files:")
for img in images.prefix(10) {
    print("  \(img.lastPathComponent)")
}
if images.count > 10 {
    print("  ... and \(images.count - 10) more")
}
