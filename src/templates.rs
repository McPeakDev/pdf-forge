//! Sample HTML templates for testing and demonstration.
//!
//! Each template exercises different supported elements and styles.

/// Simple invoice-style template with headings, paragraphs, and a table.
pub fn invoice_template() -> &'static str {
    r##"
<div class="p-6">
    <h1 class="text-3xl font-bold mb-4" style="color: #1a365d">Invoice #2024-001</h1>

    <div class="flex justify-between mb-6">
        <div>
            <p class="font-bold">From:</p>
            <p>Acme Corp</p>
            <p>123 Business St</p>
            <p>New York, NY 10001</p>
        </div>
        <div>
            <p class="font-bold">To:</p>
            <p>Client Inc</p>
            <p>456 Client Ave</p>
            <p>Los Angeles, CA 90001</p>
        </div>
    </div>

    <table class="w-full mb-6">
        <tr>
            <th class="text-left p-2 bg-gray-200">Item</th>
            <th class="text-left p-2 bg-gray-200">Qty</th>
            <th class="text-left p-2 bg-gray-200">Price</th>
            <th class="text-left p-2 bg-gray-200">Total</th>
        </tr>
        <tr>
            <td class="p-2">Web Development</td>
            <td class="p-2">40</td>
            <td class="p-2">$150.00</td>
            <td class="p-2">$6,000.00</td>
        </tr>
        <tr>
            <td class="p-2">Design Services</td>
            <td class="p-2">20</td>
            <td class="p-2">$125.00</td>
            <td class="p-2">$2,500.00</td>
        </tr>
        <tr>
            <td class="p-2">Hosting (Annual)</td>
            <td class="p-2">1</td>
            <td class="p-2">$500.00</td>
            <td class="p-2">$500.00</td>
        </tr>
    </table>

    <div class="text-right">
        <p class="text-xl font-bold">Total: $9,000.00</p>
    </div>
</div>
"##
}

/// Report template with headings, paragraphs, inline spans, and lists.
pub fn report_template() -> &'static str {
    r##"
<div class="p-6">
    <h1 class="text-3xl font-bold mb-2">Quarterly Report</h1>
    <p class="text-gray-500 mb-6">Q4 2025 — <span class="font-bold">Confidential</span></p>

    <h2 class="text-2xl font-bold mb-2">Executive Summary</h2>
    <p class="mb-4">
        Revenue grew by <span class="font-bold text-green-500">23%</span> year-over-year,
        reaching a total of <span class="font-bold">$4.2M</span> for the quarter.
        Our customer base expanded significantly with
        <span class="italic">notable wins in the enterprise segment</span>.
    </p>

    <h2 class="text-2xl font-bold mb-2">Key Highlights</h2>
    <ul class="mb-4">
        <li>Customer acquisition cost reduced by 15%</li>
        <li>Net promoter score improved to 72</li>
        <li>Three new enterprise partnerships signed</li>
        <li>Product reliability reached 99.97% uptime</li>
    </ul>

    <h2 class="text-2xl font-bold mb-2">Action Items</h2>
    <ol class="mb-4">
        <li>Expand sales team by Q1 2026</li>
        <li>Launch mobile application beta</li>
        <li>Complete SOC2 Type II certification</li>
    </ol>

    <h3 class="text-xl font-bold mb-2">Revenue Breakdown</h3>
    <table class="w-full mb-4">
        <tr>
            <th class="text-left p-2 bg-gray-200">Segment</th>
            <th class="text-left p-2 bg-gray-200">Revenue</th>
            <th class="text-left p-2 bg-gray-200">Growth</th>
        </tr>
        <tr>
            <td class="p-2">Enterprise</td>
            <td class="p-2">$2.1M</td>
            <td class="p-2 text-green-500">+31%</td>
        </tr>
        <tr>
            <td class="p-2">Mid-Market</td>
            <td class="p-2">$1.4M</td>
            <td class="p-2 text-green-500">+18%</td>
        </tr>
        <tr>
            <td class="p-2">SMB</td>
            <td class="p-2">$0.7M</td>
            <td class="p-2 text-green-500">+12%</td>
        </tr>
    </table>

    <p class="text-sm text-gray-500 mt-6">
        This document is confidential. Do not distribute without authorization.
    </p>
</div>
"##
}

/// Multi-page template that forces pagination.
pub fn multi_page_template() -> &'static str {
    r##"
<div class="p-6">
    <h1 class="text-3xl font-bold mb-4">Product Specification Document</h1>

    <h2 class="text-2xl font-bold mb-2">1. Introduction</h2>
    <p class="mb-4">
        This document provides a comprehensive specification for the next
        generation platform. It covers architecture decisions, API design,
        security requirements, and deployment considerations.
    </p>

    <h2 class="text-2xl font-bold mb-2">2. Architecture Overview</h2>
    <p class="mb-2">The system follows a microservices architecture with the following key components:</p>
    <ul class="mb-4">
        <li>API Gateway — handles authentication and rate limiting</li>
        <li>User Service — manages user accounts and permissions</li>
        <li>Order Service — processes orders and payments</li>
        <li>Notification Service — sends emails and push notifications</li>
        <li>Analytics Service — collects and processes usage telemetry</li>
    </ul>

    <h2 class="text-2xl font-bold mb-2">3. API Design</h2>
    <p class="mb-2">All APIs follow RESTful conventions with JSON payloads.</p>
    <table class="w-full mb-4">
        <tr>
            <th class="p-2 bg-gray-200">Endpoint</th>
            <th class="p-2 bg-gray-200">Method</th>
            <th class="p-2 bg-gray-200">Description</th>
        </tr>
        <tr><td class="p-2">/api/users</td><td class="p-2">GET</td><td class="p-2">List users</td></tr>
        <tr><td class="p-2">/api/users</td><td class="p-2">POST</td><td class="p-2">Create user</td></tr>
        <tr><td class="p-2">/api/orders</td><td class="p-2">GET</td><td class="p-2">List orders</td></tr>
        <tr><td class="p-2">/api/orders</td><td class="p-2">POST</td><td class="p-2">Create order</td></tr>
        <tr><td class="p-2">/api/orders/:id</td><td class="p-2">PUT</td><td class="p-2">Update order</td></tr>
        <tr><td class="p-2">/api/orders/:id</td><td class="p-2">DELETE</td><td class="p-2">Cancel order</td></tr>
        <tr><td class="p-2">/api/notifications</td><td class="p-2">GET</td><td class="p-2">List notifications</td></tr>
        <tr><td class="p-2">/api/notifications</td><td class="p-2">POST</td><td class="p-2">Send notification</td></tr>
    </table>

    <h2 class="text-2xl font-bold mb-2">4. Security Requirements</h2>
    <p class="mb-2">The platform must meet the following security standards:</p>
    <ol class="mb-4">
        <li>All data at rest must be encrypted using AES-256</li>
        <li>All data in transit must use TLS 1.3</li>
        <li>Authentication via OAuth 2.0 with PKCE</li>
        <li>Role-based access control (RBAC) for all endpoints</li>
        <li>Audit logging for all state mutations</li>
        <li>Rate limiting at the API gateway level</li>
    </ol>

    <h2 class="text-2xl font-bold mb-2">5. Deployment</h2>
    <p class="mb-2">
        The system will be deployed on Kubernetes with the following resource allocations:
    </p>
    <table class="w-full mb-4">
        <tr>
            <th class="p-2 bg-gray-200">Service</th>
            <th class="p-2 bg-gray-200">CPU</th>
            <th class="p-2 bg-gray-200">Memory</th>
            <th class="p-2 bg-gray-200">Replicas</th>
        </tr>
        <tr><td class="p-2">API Gateway</td><td class="p-2">2 cores</td><td class="p-2">4 GB</td><td class="p-2">3</td></tr>
        <tr><td class="p-2">User Service</td><td class="p-2">1 core</td><td class="p-2">2 GB</td><td class="p-2">2</td></tr>
        <tr><td class="p-2">Order Service</td><td class="p-2">2 cores</td><td class="p-2">4 GB</td><td class="p-2">3</td></tr>
        <tr><td class="p-2">Notification Service</td><td class="p-2">0.5 cores</td><td class="p-2">1 GB</td><td class="p-2">2</td></tr>
        <tr><td class="p-2">Analytics Service</td><td class="p-2">4 cores</td><td class="p-2">8 GB</td><td class="p-2">2</td></tr>
    </table>

    <h2 class="text-2xl font-bold mb-2">6. Monitoring and Observability</h2>
    <p class="mb-4">
        All services emit structured logs, metrics, and distributed traces.
        A centralized observability stack (Prometheus, Grafana, Jaeger) provides
        dashboards, alerting, and end-to-end trace correlation.
    </p>

    <h2 class="text-2xl font-bold mb-2">7. Conclusion</h2>
    <p class="mb-4">
        This specification provides the foundation for the next generation of our
        platform. All teams should review and provide feedback by end of Q1 2026.
    </p>

    <p class="text-xs text-gray-500">Document version 1.0 — Last updated February 2026</p>
</div>
"##
}

/// Template with images and mixed inline styles.
pub fn styled_template() -> &'static str {
    r##"
<div class="p-6">
    <div class="flex items-center mb-6">
        <img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==" style="width: 64px; height: 64px" />
        <div class="ml-4">
            <h1 class="text-2xl font-bold">Company Name</h1>
            <p class="text-gray-500 text-sm">Building the future</p>
        </div>
    </div>

    <h2 class="text-xl font-bold mb-2" style="color: #2b6cb0">Product Overview</h2>
    <p class="mb-4">
        Our <span class="font-bold">flagship product</span> combines
        <span class="italic">cutting-edge technology</span> with
        <span class="underline">user-friendly design</span>.
        It handles <span style="color: #e53e3e">critical workflows</span>
        across multiple industries.
    </p>

    <h3 class="text-lg font-bold mb-2">Features</h3>
    <div class="flex gap-4 mb-4">
        <div class="flex-1 p-4 bg-gray-100">
            <p class="font-bold mb-1">Fast</p>
            <p class="text-sm">Sub-second response times</p>
        </div>
        <div class="flex-1 p-4 bg-gray-100">
            <p class="font-bold mb-1">Secure</p>
            <p class="text-sm">Enterprise-grade encryption</p>
        </div>
        <div class="flex-1 p-4 bg-gray-100">
            <p class="font-bold mb-1">Scalable</p>
            <p class="text-sm">Handles millions of requests</p>
        </div>
    </div>

    <h3 class="text-lg font-bold mb-2">Pricing</h3>
    <table class="w-full">
        <tr>
            <th class="p-2 bg-gray-200">Plan</th>
            <th class="p-2 bg-gray-200">Users</th>
            <th class="p-2 bg-gray-200">Price</th>
        </tr>
        <tr>
            <td class="p-2 font-bold">Starter</td>
            <td class="p-2">Up to 10</td>
            <td class="p-2">$29/mo</td>
        </tr>
        <tr>
            <td class="p-2 font-bold">Pro</td>
            <td class="p-2">Up to 100</td>
            <td class="p-2">$99/mo</td>
        </tr>
        <tr>
            <td class="p-2 font-bold">Enterprise</td>
            <td class="p-2">Unlimited</td>
            <td class="p-2">Contact us</td>
        </tr>
    </table>
</div>
"##
}

/// Minimal template for unit testing.
pub fn minimal_template() -> &'static str {
    r#"<div><h1>Title</h1><p>Body text</p></div>"#
}

/// Template exercising all supported elements.
pub fn all_elements_template() -> &'static str {
    r##"
<div class="p-4">
    <h1 class="text-3xl font-bold">Heading 1</h1>
    <h2 class="text-2xl font-bold">Heading 2</h2>
    <h3 class="text-xl font-bold">Heading 3</h3>

    <p class="mb-4">
        This is a paragraph with <span class="font-bold">bold</span>,
        <span class="italic">italic</span>, and
        <span class="underline">underlined</span> text.
    </p>

    <ul class="mb-4">
        <li>Unordered item 1</li>
        <li>Unordered item 2</li>
        <li>Unordered item 3</li>
    </ul>

    <ol class="mb-4">
        <li>Ordered item 1</li>
        <li>Ordered item 2</li>
        <li>Ordered item 3</li>
    </ol>

    <table class="w-full mb-4">
        <tr>
            <th class="p-2 bg-gray-200">Header A</th>
            <th class="p-2 bg-gray-200">Header B</th>
        </tr>
        <tr>
            <td class="p-2">Cell A1</td>
            <td class="p-2">Cell B1</td>
        </tr>
        <tr>
            <td class="p-2">Cell A2</td>
            <td class="p-2">Cell B2</td>
        </tr>
    </table>

    <img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==" style="width: 100px; height: 60px" />
</div>
"##
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_are_valid_html() {
        let templates: Vec<(&str, &str)> = vec![
            ("invoice", invoice_template()),
            ("report", report_template()),
            ("multipage", multi_page_template()),
            ("styled", styled_template()),
            ("minimal", minimal_template()),
            ("all_elements", all_elements_template()),
        ];

        for (name, html) in templates {
            let dom = crate::dom::parse_html(html);
            assert!(
                !dom.is_empty(),
                "Template '{}' should parse to non-empty DOM",
                name
            );
        }
    }
}
