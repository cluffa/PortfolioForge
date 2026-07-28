# PDF editor spec

Project Specification

PortfolioForge

1. Project Overview

PortfolioForge is a privacy-first document packaging application that creates and edits Adobe-compatible PDF Portfolios without requiring Adobe Acrobat.

The application converts common document formats into PDFs, organizes them into a professional portfolio, and performs all processing locally on the user's device.

The primary focus is not general PDF editing. The product specializes in creating, managing, and maintaining document collections packaged as PDF Portfolios.

---

2. Vision

Provide a lightweight alternative for professionals who need to assemble document packages without purchasing Adobe Acrobat.

Users should be able to:

- Collect documents.

- Convert them into a consistent PDF format.

- Organize them.

- Create a portable PDF Portfolio.

- Edit existing portfolios.

- Maintain privacy by keeping all processing local.

---

3. Product Principles

Privacy First

- No document uploads.

- No cloud processing.

- No accounts required.

- No document analytics.

- All processing occurs locally.

Compatibility First

Generated portfolios should open correctly in:

- Adobe Acrobat

- Adobe Reader

- Common PDF viewers where supported

Lightweight Design

Avoid becoming a full PDF editor.

Focus on:

- Packaging

- Conversion

- Organization

- Portfolio management

---

4. Primary Use Cases

Professional Document Packages

Examples:

- Legal case files

- Financial records

- Insurance claims

- Engineering submissions

- Compliance packages

- Client deliverables

- Audit documentation

---

5. Version 1 Scope

Supported Input Files

Required

- PDF

- DOCX

- PNG

- JPG

- JPEG

- TIFF

Optional

- BMP

- WebP

- GIF

Not Supported Initially

- DOC

- XLS/XLSX

- PPT/PPTX

- Audio

- Video

- Executable files

---

6. Conversion Rules

All portfolio contents are stored as PDFs.

Examples

Document.docx

becomes

Document.pdf

Image.jpg

becomes

Image.pdf

Existing PDF files are preserved without modification.

---

7. User Workflow

Create Portfolio

1. User opens application.

2. User drags files or folders into the application.

3. Application analyzes files.

4. Unsupported files are reported.

5. Supported files are converted to PDF.

6. User arranges document order.

7. User optionally creates a cover page.

8. Portfolio is generated.

9. User downloads the completed PDF Portfolio.

---

Edit Existing Portfolio

1. User opens existing portfolio.

2. Application reads portfolio structure.

3. Embedded documents are displayed.

4. User can:

   - add documents

   - remove documents

   - replace documents

   - reorder documents

   - rename entries

5. Updated portfolio is saved.

---

8. Technical Architecture

Overall Design

                 Web Interface

                      |

                      |

              Rust WebAssembly

                      |

        ------------------------------

        |            |               |

 Portfolio Engine  Conversion    Validation

                  Engine

---

9. Technology Stack

Core Engine

Language:

Rust

Responsibilities:

- PDF Portfolio parsing

- PDF Portfolio creation

- PDF Portfolio modification

- PDF generation

- Image conversion

- Document conversion

- Validation

Output:

- WebAssembly module

- Native Rust library

---

Web Application

Technologies:

- TypeScript

- React

- Vite

- Tailwind CSS

Responsibilities:

- User interface

- Drag and drop

- File selection

- Progress reporting

- Preview

- Export

---

10. Repository Structure

portfolioforge/

├── apps/

│   └── web/

│

├── crates/

│   ├── portfolio-core/

│   ├── portfolio-parser/

│   ├── portfolio-writer/

│   ├── pdf-generator/

│   ├── image-converter/

│   ├── docx-converter/

│   ├── cover-generator/

│   └── validator/

│

├── tests/

│

├── samples/

│

└── documentation/

---

11. Development Phases

Phase 0 — Research

Objective

Understand PDF Portfolio structure and requirements.

Tasks

- Analyze portfolios created by Acrobat.

- Document PDF object structures.

- Identify required components.

- Separate PDF standards from Adobe-specific features.

- Build a portfolio sample library.

Deliverable

PDF Portfolio technical specification.

---

Phase 0.5 — Proof of Feasibility

Purpose

Validate the highest-risk assumptions before full development.

The two critical questions:

1. Can a compatible PDF Portfolio be generated without Adobe-provided templates or proprietary assets?

2. Can existing PDF Portfolios be modified while remaining compatible with Acrobat?

---

Portfolio Generation Test

Create a minimal portfolio containing:

- PDF catalog

- Collection dictionary

- Embedded file collection

- Metadata

- Document references

Validate with:

- Adobe Acrobat

- Adobe Reader

Success criteria:

- Opens correctly.

- Documents are accessible.

- No Adobe assets are required.

---

Portfolio Editing Test

Modify an existing portfolio:

- Read contents.

- Add PDF.

- Remove PDF.

- Rewrite structure.

Success criteria:

- Opens in Acrobat.

- Documents remain accessible.

- Structure remains valid.

---

Conversion Test

Validate:

- Image-to-PDF conversion.

- DOCX-to-PDF conversion.

- Browser performance.

- WebAssembly feasibility.

---

Deliverables

- Portfolio generator prototype.

- Portfolio modification prototype.

- Conversion prototype.

- Compatibility report.

- Development decision.

---

Go / No-Go Criteria

Continue if:

- Portfolios can be generated.

- Portfolios can be modified.

- Acrobat compatibility is maintained.

- Conversion is practical.

Reassess if:

- Proprietary Adobe components are required.

- Browser limitations prevent reliable operation.

---

Phase 1 — Core Reader

Features

- Detect PDF Portfolio files.

- Read metadata.

- Enumerate embedded documents.

- Extract embedded files.

Deliverable

Rust library capable of reading portfolio structures.

---

Phase 2 — Portfolio Writer

Features

- Add PDFs.

- Remove PDFs.

- Update metadata.

- Save valid portfolios.

Deliverable

Portfolio creation/modification engine.

---

Phase 3 — Conversion Engine

Features

Image conversion:

- PNG

- JPG

- TIFF

- WebP

Document conversion:

- DOCX to PDF

Deliverable

Automated conversion pipeline.

---

Phase 4 — Portfolio Creation

Features

- Generate blank portfolios.

- Create new portfolios.

- Add converted documents.

- Generate metadata.

- Validate output.

Deliverable

Complete portfolio generation system.

---

Phase 5 — Web Application

Features

- Drag-and-drop interface.

- File queue.

- Conversion progress.

- Document ordering.

- Preview.

- Download.

Deployment

Initial deployment:

- GitHub Pages

Requirements:

- Static hosting only.

- Client-side processing.

---

Phase 6 — Portfolio Editing

Features

- Open existing portfolios.

- Add files.

- Remove files.

- Replace files.

- Rename files.

- Reorder documents.

- Save updates.

---

Phase 7 — Testing and Release

Compatibility Testing

Platforms:

- Chrome

- Edge

- Firefox

- Safari

PDF Validation:

- Adobe Acrobat

- Adobe Reader

---

Performance Testing

Targets:

- 100+ documents

- 500 MB portfolios

- Large image sets

---

12. Security Requirements

The application must:

- Process files locally.

- Avoid uploading user documents.

- Avoid storing documents remotely.

- Avoid telemetry.

- Validate input files before processing.

---

13. Performance Targets

Startup:

Less than 3 seconds.

Conversion:

Approximately one second per standard image.

Portfolio creation:

Suitable for professional document packages.

---

14. Future Roadmap

Version 2

- Optional preservation of original file types.

- Folder hierarchy support.

- OCR.

- Password protection.

- Advanced metadata editing.

- Search.

---

Version 3

- Desktop application using Tauri.

- Command-line version.

- Enterprise batch processing.

- API access.

- Custom branding.

- Automated portfolio templates.

---

15. Success Criteria

PortfolioForge succeeds when users can:

- Create PDF Portfolios without Acrobat.

- Convert common documents into PDFs.

- Edit existing portfolios.

- Process files privately in their browser.

- Generate files that open correctly in Acrobat.

- Use the product without installing software.

---

16. Long-Term Vision

PortfolioForge becomes a reusable document packaging platform.

The Rust core serves as the foundation for:

- Browser applications.

- Desktop applications.

- Command-line tools.

- Enterprise integrations.

The product remains focused on one problem: creating professional document packages quickly, privately, and without requiring expensive proprietary PDF software.